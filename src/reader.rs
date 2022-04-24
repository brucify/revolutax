use csv::{ReaderBuilder, Trim, WriterBuilder};
use log::{debug};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{self};
use std::path::PathBuf;
// use chrono::NaiveDateTime;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Transaction {
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
    currency: String,
    #[serde(rename = "Original Amount")]
    original_amount: Decimal,
    #[serde(rename = "Original Currency")]
    original_currency: String,
    #[serde(rename = "Settled Amount")]
    settled_amount: Option<Decimal>,
    #[serde(rename = "Settled Currency")]
    settled_currency: Option<String>,
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
pub struct Account {
    #[serde(rename = "client")]
    client_id:  u16,
    available:  Decimal,
    held:       Decimal,
    total:      Decimal,
    locked:     bool,
}

/// Reads the file from path into an ordered `Vec<Transaction>`.
async fn deserialize_from_path(path: &PathBuf) -> io::Result<Vec<Transaction>> {
    let now = std::time::Instant::now();
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        // .delimiter(b';')
        .delimiter(b',')
        .trim(Trim::All)
        .from_path(path)?;
    debug!("ReaderBuilder::from_path done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let txns: Vec<Transaction> =
        rdr.deserialize::<Transaction>()
            .filter_map(|record| record.ok())
            .collect();
    debug!("reader::deserialize done. Elapsed: {:.2?}", now.elapsed());

    Ok(txns)
}

pub async fn read_exchanges(path: &PathBuf) -> io::Result<Vec<Transaction>> {
    let txns = deserialize_from_path(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .collect();
    Ok(txns)
}

pub async fn read_exchanges_in_currency(path: &PathBuf, currency: String) -> io::Result<Vec<Transaction>> {
    let txns = deserialize_from_path(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .filter(|t| t.currency.eq(&currency) || t.description.contains(&currency))// "Exchanged to ETH"
        .collect();
    Ok(txns)
}

/// Wraps the `stdout.lock()` in a `csv::Writer` and writes the accounts.
/// The `csv::Writer` is already buffered so there is no need to wrap
/// `stdout.lock()` in a `io::BufWriter`.
pub async fn print_transactions(txns: &Vec<Transaction>) -> io::Result<()>{
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
