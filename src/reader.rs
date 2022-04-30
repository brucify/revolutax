use crate::transaction::{Currency, Transaction, TransactionType};
use csv::{ReaderBuilder, Trim, WriterBuilder};
use log::{debug};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{self};
use std::path::PathBuf;
// use chrono::NaiveDateTime;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct Row {
    #[serde(rename = "Type")]
    r#type: Type,
    #[serde(rename = "Started Date")]
    started_date: String,
    #[serde(rename = "Completed Date")]
    completed_date: Option<String>,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Amount")]
    amount: Decimal,
    #[serde(rename = "Fee")]
    fee: Decimal,
    #[serde(rename = "Currency")]
    currency: Currency,
    #[serde(rename = "Original Amount")]
    original_amount: Decimal,
    #[serde(rename = "Original Currency")]
    original_currency: Currency,
    #[serde(rename = "Settled Amount")]
    settled_amount: Option<Decimal>,
    #[serde(rename = "Settled Currency")]
    settled_currency: Option<Currency>,
    #[serde(rename = "State")]
    state: State,
    #[serde(rename = "Balance")]
    balance: Option<Decimal>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum Type {
    Exchange,
    Transfer,
    Cashback,
    #[serde(rename = "Card Payment")]
    CardPayment,
    Topup,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum State {
    Completed
}

#[derive(Debug, Serialize, PartialEq)]
struct Account {
    #[serde(rename = "client")]
    client_id:  u16,
    available:  Decimal,
    held:       Decimal,
    total:      Decimal,
    locked:     bool,
}

/// Reads the file from path into an ordered `Vec<Transaction>`.
async fn deserialize_from_path(path: &PathBuf) -> io::Result<Vec<Row>> {
    let now = std::time::Instant::now();
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        // .delimiter(b';')
        .delimiter(b',')
        .trim(Trim::All)
        .from_path(path)?;
    debug!("ReaderBuilder::from_path done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let txns: Vec<Row> =
        rdr.deserialize::<Row>()
            .filter_map(|record| record.ok())
            .collect();
    debug!("reader::deserialize done. Elapsed: {:.2?}", now.elapsed());

    Ok(txns)
}

pub(crate) async fn read_exchanges(path: &PathBuf) -> io::Result<Vec<Row>> {
    let txns = deserialize_from_path(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .collect();
    Ok(txns)
}

pub(crate) async fn read_exchanges_in_currency(path: &PathBuf, currency: &Currency) -> io::Result<Vec<Row>> {
    let txns = deserialize_from_path(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .filter(|t| t.currency.eq(currency) || t.description.contains(currency))// "Exchanged to ETH"
        .collect();
    Ok(txns)
}

pub(crate) async fn to_transactions(txns: &Vec<Row>, currency: &Currency) -> io::Result<Vec<Transaction>> {
    let (txns, _): (Vec<Transaction>, Option<&Row>) =
        txns.iter().rev()
            .fold((vec![], None), |(mut acc, prev), row| {
                match prev {
                    None => (acc, Some(row)),
                    Some(prev) => {
                        let txn = prev.to_transaction(None, currency);
                        let txn = row.to_transaction(Some(txn), currency);
                        acc.push(txn);
                        (acc, None)
                    }
                }
            });
    Ok(txns)
}

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
impl Row {
    fn to_transaction(&self, txn: Option<Transaction>, currency: &Currency) -> Transaction {
        let mut txn = txn.unwrap_or(Transaction::new());

        // target currency: "BCH", currency: "BCH", description: "Exchanged from SEK"
        if self.currency.eq(currency) && self.description.contains("Exchanged from") {
            debug!("{:?}: Bought {:?} of {:?} ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            txn.r#type = TransactionType::Buy;
            txn.paid_amount = self.amount + self.fee;
            txn.paid_currency = currency.clone();
            txn.date = self.started_date.clone();
        }
        // target currency: "BCH", currency: "BCH", description: "Exchanged to SEK"
        if self.currency.eq(currency) && self.description.contains("Exchanged to") {
            debug!("{:?}: Sold {:?} of {:?} ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            txn.r#type = TransactionType::Sell;
            txn.paid_amount = self.amount + self.fee;
            txn.paid_currency = currency.clone();
            txn.date = self.started_date.clone();
        }
        // target currency: "BCH", currency: "SEK", description: "Exchanged from BCH"
        if self.description.contains("Exchanged from") && self.description.contains(currency) {
            debug!("{:?}: Income of selling is the price of {:?} of {:?} in SEK ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            txn.r#type = TransactionType::Sell;
            txn.exchanged_amount = self.amount + self.fee;
            txn.exchanged_currency = self.currency.clone();
        }
        // target currency: "BCH", currency: "SEK", description: "Exchanged to BCH"
        if self.description.contains("Exchanged to") && self.description.contains(currency) {
            debug!("{:?}: Cost of buying is the price of {:?} of {:?} in SEK ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            txn.r#type = TransactionType::Buy;
            txn.exchanged_amount = self.amount + self.fee;
            txn.exchanged_currency = self.currency.clone();
        }
        if self.description.contains("Vault") {
            txn.is_vault = true;
        }
        txn
    }
}

/// Wraps the `stdout.lock()` in a `csv::Writer` and writes the accounts.
/// The `csv::Writer` is already buffered so there is no need to wrap
/// `stdout.lock()` in a `io::BufWriter`.
pub(crate) async fn print_rows(txns: &Vec<Row>) -> io::Result<()>{
    let stdout = io::stdout();
    let lock = stdout.lock();
    let mut wtr =
        WriterBuilder::new()
            .has_headers(true)
            .from_writer(lock);

    let mut err = None;
    txns.iter().for_each(|t|
        wtr.serialize(t)
            .unwrap_or_else(|e| {
                err = Some(e);
                Default::default()
            })
    );
    err.map_or(Ok(()), Err)?;
    Ok(())
}

#[cfg(test)]
mod test {

    #[test]
    fn test_read_with() -> Result<(), anyhow::Error> {
        Ok(())
    }
}