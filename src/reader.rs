use crate::calculator::{Currency, Trade, Direction};
use csv::{ReaderBuilder, Trim};
use log::{debug, info};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{self};
use std::ops::Neg;
use std::path::PathBuf;

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
    Topup,

    #[serde(rename = "Card Payment")]
    CardPayment,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum State {
    Completed,
    Declined,
}

/// Reads the file from path into a `Vec<Row>`.
async fn deserialize_from(path: &PathBuf) -> io::Result<Vec<Row>> {
    let now = std::time::Instant::now();
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        // .delimiter(b';')
        .delimiter(b',')
        .trim(Trim::All)
        .from_path(path)?;
    info!("ReaderBuilder::from_path done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let rows: Vec<Row> =
        rdr.deserialize::<Row>()
            .filter_map(|record| record.ok())
            .collect();
    info!("reader::deserialize done. Elapsed: {:.2?}", now.elapsed());

    Ok(rows)
}

/// Reads the file from path into a `Vec<Row>`, returns only rows with type `Exchange`.
pub(crate) async fn read_exchanges(path: &PathBuf) -> io::Result<Vec<Row>> {
    let rows = deserialize_from(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .collect();
    Ok(rows)
}

/// Reads the file from path into a `Vec<Row>`, returns only rows with type `Exchange` in the
/// target currency, or  with type `Card Payment` but in the target currency.
pub(crate) async fn read_exchanges_in_currency(path: &PathBuf, currency: &Currency) -> io::Result<Vec<Row>> {
    let rows = deserialize_from(path).await?
        .into_iter()
        .filter(|t| {
            t.r#type == Type::Exchange
            || (t.r#type == Type::CardPayment && t.currency.eq(currency))
        })
        .filter(|t| t.state == State::Completed)
        .filter(|t| t.currency.eq(currency) || t.description.contains(currency))// "Exchanged to ETH"
        .collect();
    Ok(rows)
}

/// Converts `Vec<Row>` into `Vec<Trade>`, given a target currency.
pub(crate) async fn to_trades(rows: &Vec<Row>, currency: &Currency) -> io::Result<Vec<Trade>> {
    let (trades, _): (Vec<Trade>, Option<&Row>) =
        rows.iter().rev()
            .fold((vec![], None), |(mut acc, prev), row| {
                match row.r#type {
                    Type::Exchange => {
                        match prev {
                            None => (acc, Some(row)),
                            Some(prev) => {
                                let trade = prev.to_trade(None, currency);
                                let trade = row.to_trade(Some(trade), currency);
                                acc.push(trade);
                                (acc, None)
                            }
                        }
                    }
                    Type::CardPayment => {
                        let trade = row.to_trade(None, currency);
                        acc.push(trade);
                        (acc, prev)
                    }
                    _ => (acc, prev)
                }
            });
    Ok(trades)
}

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
impl Row {
    fn to_trade(&self, trade: Option<Trade>, currency: &Currency) -> Trade {
        let mut trade = trade.unwrap_or(Trade::new());

        match self.r#type {
            Type::Exchange => self.exchange_to_trade(&mut trade, currency),
            Type::CardPayment => self.card_payment_to_trade(&mut trade, currency),
            _ => {}
        }

        trade
    }

    fn exchange_to_trade(&self, trade: &mut Trade, currency: &Currency) {
        // target currency: "BCH", currency: "BCH", description: "Exchanged from SEK"
        // if self.currency.eq(currency) && self.description.contains("Exchanged from") {
        if self.currency.eq(currency) && self.amount.is_sign_positive() {
            debug!("{:?}: Bought {:?} of {:?} ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            trade.direction = Direction::Buy;
            trade.paid_amount = self.amount + self.fee;
            trade.paid_currency = currency.clone();
            trade.date = self.started_date.clone();

        }
        // target currency: "BCH", currency: "BCH", description: "Exchanged to SEK"
        // if self.currency.eq(currency) && self.description.contains("Exchanged to") {
        if self.currency.eq(currency) && self.amount.is_sign_negative() {
            debug!("{:?}: Sold {:?} of {:?} ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            trade.direction = Direction::Sell;
            trade.paid_amount = self.amount + self.fee;
            trade.paid_currency = currency.clone();
            trade.date = self.started_date.clone();
        }
        // target currency: "BCH", currency: "SEK", description: "Exchanged from BCH"
        if self.description.contains("Exchanged from") && self.description.contains(currency) {
            debug!("{:?}: Income of selling is the price of {:?} of {:?} in SEK ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            trade.direction = Direction::Sell;
            trade.exchanged_amount = self.amount + self.fee;
            trade.exchanged_currency = self.currency.clone();
        }
        // target currency: "BCH", currency: "SEK", description: "Exchanged to BCH"
        if self.description.contains("Exchanged to") && self.description.contains(currency) {
            debug!("{:?}: Cost of buying is the price of {:?} of {:?} in SEK ({:?}), incl. fee {:?}", self.started_date, self.amount+self.fee, self.currency, self.description, self.fee);
            trade.direction = Direction::Buy;
            trade.exchanged_amount = self.amount + self.fee;
            trade.exchanged_currency = self.currency.clone();
        }
        if self.description.contains("Vault") {
            trade.is_vault = true;
        }
    }

    fn card_payment_to_trade(&self, trade: &mut Trade, currency: &Currency) {
        // amount: -0.00123456, fee: 0.00000000, currency: "BTC", original_amount: -543.21, original_currency: "SEK",
        // settled_amount: Some(543.21), settled_currency: Some("SEK"), state: Completed, balance: Some(0.00000000) }
        trade.direction = Direction::Sell;
        trade.paid_amount = self.amount + self.fee;
        trade.paid_currency = currency.clone();
        trade.exchanged_amount = self.original_amount.neg();
        trade.exchanged_currency = self.original_currency.clone();
        trade.date = self.started_date.clone();
        trade.is_vault = false;
    }
}

#[cfg(test)]
mod test {
    use crate::reader::*;
    use futures::executor::block_on;
    use rust_decimal_macros::dec;
    use std::error::Error;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn should_deserialize_from_path() -> Result<(), Box<dyn Error>> {
        /*
         * Given
         */
        let mut file = NamedTempFile::new()?;
        writeln!(file, "Type,Started Date,Completed Date,Description,Amount,Fee,Currency,Original Amount,Original Currency,Settled Amount,Settled Currency,State,Balance
                        Exchange,2022-03-01 16:21:49,2022-03-01 16:21:49,Exchanged to EOS,-900.90603463,-20.36495977,DOGE,-900.90603463,DOGE,,,Completed,1078.7290056
                        Exchange,2022-03-01 16:21:49,2022-03-01 16:21:49,Exchanged from DOGE,50,0,EOS,50,EOS,,,Completed,50
                        Exchange,2021-12-31 17:54:48,2021-12-31 17:54:48,Exchanged to DOGE,-5000.45,-80.15,SEK,-5000.45,SEK,,,Completed,700.27
                        Exchange,2021-12-31 17:54:48,2021-12-31 17:54:48,Exchanged from SEK,2000,0,DOGE,2000,DOGE,,,Completed,2000")?;
        let path = file.path().to_str().unwrap();

        /*
         * When
         */
        let rows = block_on(deserialize_from(&PathBuf::from(path)))?;

        /*
         * Then
         */
        let mut iter = rows.into_iter();
        assert_eq!(iter.next(), Some(Row{
            r#type: Type::Exchange,
            started_date: "2022-03-01 16:21:49".to_string(),
            completed_date: Some("2022-03-01 16:21:49".to_string()),
            description: "Exchanged to EOS".to_string(),
            amount: dec!(-900.90603463),
            fee: dec!(-20.36495977),
            currency: "DOGE".to_string(),
            original_amount: dec!(-900.90603463),
            original_currency: "DOGE".to_string(),
            settled_amount: None,
            settled_currency: None,
            state: State::Completed,
            balance: Some(dec!(1078.7290056))
        }));
        assert_eq!(iter.next(), Some(Row{
            r#type: Type::Exchange,
            started_date: "2022-03-01 16:21:49".to_string(),
            completed_date: Some("2022-03-01 16:21:49".to_string()),
            description: "Exchanged from DOGE".to_string(),
            amount: dec!(50),
            fee: dec!(0),
            currency: "EOS".to_string(),
            original_amount: dec!(50),
            original_currency: "EOS".to_string(),
            settled_amount: None,
            settled_currency: None,
            state: State::Completed,
            balance: Some(dec!(50))
        }));
        assert_eq!(iter.next(), Some(Row{
            r#type: Type::Exchange,
            started_date: "2021-12-31 17:54:48".to_string(),
            completed_date: Some("2021-12-31 17:54:48".to_string()),
            description: "Exchanged to DOGE".to_string(),
            amount: dec!(-5000.45),
            fee: dec!(-80.15),
            currency: "SEK".to_string(),
            original_amount: dec!(-5000.45),
            original_currency: "SEK".to_string(),
            settled_amount: None,
            settled_currency: None,
            state: State::Completed,
            balance: Some(dec!(700.27))
        }));
        assert_eq!(iter.next(), Some(Row{
            r#type: Type::Exchange,
            started_date: "2021-12-31 17:54:48".to_string(),
            completed_date: Some("2021-12-31 17:54:48".to_string()),
            description: "Exchanged from SEK".to_string(),
            amount: dec!(2000),
            fee: dec!(0),
            currency: "DOGE".to_string(),
            original_amount: dec!(2000),
            original_currency: "DOGE".to_string(),
            settled_amount: None,
            settled_currency: None,
            state: State::Completed,
            balance: Some(dec!(2000))
        }));
        assert_eq!(iter.next(), None);
        Ok(())
    }

    #[test]
    fn should_parse_trades_from_rows() -> Result<(), Box<dyn Error>> {
        /*
         * Given
         */
        let rows = vec![
            Row{
                r#type: Type::CardPayment,
                started_date: "2022-04-02 17:22:50".to_string(),
                completed_date: Some("2022-04-02 17:22:50".to_string()),
                description: "Klarna".to_string(),
                amount: dec!(-123.45678901),
                fee: dec!(0.00000000),
                currency: "DOGE".to_string(),
                original_amount: dec!(-321.23456789),
                original_currency: "SEK".to_string(),
                settled_amount: Some(dec!(321.23456789)),
                settled_currency: Some("SEK".to_string()),
                state: State::Completed,
                balance: Some(dec!(9876.123345))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2022-03-01 16:21:49".to_string(),
                completed_date: Some("2022-03-01 16:21:49".to_string()),
                description: "Exchanged to EOS".to_string(),
                amount: dec!(-900.90603463),
                fee: dec!(-20.36495977),
                currency: "DOGE".to_string(),
                original_amount: dec!(-900.90603463),
                original_currency: "DOGE".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(1078.7290056))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2022-03-01 16:21:49".to_string(),
                completed_date: Some("2022-03-01 16:21:49".to_string()),
                description: "Exchanged from DOGE".to_string(),
                amount: dec!(50),
                fee: dec!(0),
                currency: "EOS".to_string(),
                original_amount: dec!(50),
                original_currency: "EOS".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(50))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-12-31 17:54:48".to_string(),
                completed_date: Some("2021-12-31 17:54:48".to_string()),
                description: "Exchanged to DOGE".to_string(),
                amount: dec!(-5000.45),
                fee: dec!(-80.15),
                currency: "SEK".to_string(),
                original_amount: dec!(-5000.45),
                original_currency: "SEK".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(700.27))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-12-31 17:54:48".to_string(),
                completed_date: Some("2021-12-31 17:54:48".to_string()),
                description: "Exchanged from SEK".to_string(),
                amount: dec!(2000),
                fee: dec!(0),
                currency: "DOGE".to_string(),
                original_amount: dec!(2000),
                original_currency: "DOGE".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(2000))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-11-11 18:03:13".to_string(),
                completed_date: Some("2021-11-11 18:03:13".to_string()),
                description: "Exchanged to DOGE DOGE Vault".to_string(),
                amount: dec!(-20),
                fee: dec!(0),
                currency: "SEK".to_string(),
                original_amount: dec!(-20),
                original_currency: "SEK".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(500))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-11-11 18:03:13".to_string(),
                completed_date: Some("2021-11-11 18:03:13".to_string()),
                description: "Exchanged from SEK".to_string(),
                amount: dec!(40),
                fee: dec!(-0.06),
                currency: "DOGE".to_string(),
                original_amount: dec!(40),
                original_currency: "DOGE".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(139.94))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-11-10 17:03:13".to_string(),
                completed_date: Some("2021-11-10 17:03:13".to_string()),
                description: "Exchanged to DOGE DOGE Vault".to_string(),
                amount: dec!(-300),
                fee: dec!(0),
                currency: "SEK".to_string(),
                original_amount: dec!(-300),
                original_currency: "SEK".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(0))
            },
            Row{
                r#type: Type::Exchange,
                started_date: "2021-11-10 17:03:13".to_string(),
                completed_date: Some("2021-11-10 17:03:13".to_string()),
                description: "".to_string(),
                amount: dec!(3),
                fee: dec!(-0.06),
                currency: "DOGE".to_string(),
                original_amount: dec!(3),
                original_currency: "DOGE".to_string(),
                settled_amount: None,
                settled_currency: None,
                state: State::Completed,
                balance: Some(dec!(200))
            }
        ];
        /*
         * When
         */
        let trades = block_on(to_trades(&rows, &"DOGE".to_string()))?;

        /*
        * Then
        */
        let mut iter = trades.into_iter();
        assert_eq!(iter.next(), Some(Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(2.94),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-300),
            date: "2021-11-10 17:03:13".to_string(),
            is_vault: true
        }));
        assert_eq!(iter.next(), Some(Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(39.94),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-20),
            date: "2021-11-11 18:03:13".to_string(),
            is_vault: true
        }));
        assert_eq!(iter.next(), Some(Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(2000),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-5080.60),
            date: "2021-12-31 17:54:48".to_string(),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(Trade {
            direction: Direction::Sell,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(-921.27099440),
            exchanged_currency: "EOS".to_string(),
            exchanged_amount: dec!(50),
            date: "2022-03-01 16:21:49".to_string(),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(Trade {
            direction: Direction::Sell,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(-123.45678901),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(321.23456789),
            date: "2022-04-02 17:22:50".to_string(),
            is_vault: false
        }));
        assert_eq!(iter.next(), None);

        Ok(())
    }
}