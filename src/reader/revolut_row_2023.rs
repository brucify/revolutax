use csv::{ReaderBuilder, Trim};
use log::info;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Result;
use std::ops::Neg;
use std::path::PathBuf;

use crate::calculator::{Currency, Direction, Trade};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct RevolutRow2023 {
    #[serde(rename = "Type")]
    r#type: Type,

    #[serde(rename = "Product")]
    product: Product,

    #[serde(rename = "Started Date")]
    started_date: String,

    #[serde(rename = "Completed Date")]
    completed_date: Option<String>,

    #[serde(rename = "Description")]
    description: String,

    #[serde(rename = "Amount")]
    amount: Decimal,

    #[serde(rename = "Currency")]
    currency: Currency,

    #[serde(rename = "Fiat amount")]
    fiat_amount: Decimal,

    #[serde(rename = "Fiat amount (inc. fees)")]
    fiat_amount_inc_fees: Decimal,

    #[serde(rename = "Fee")]
    fee: Decimal,

    #[serde(rename = "Base currency")]
    base_currency: Currency,

    #[serde(rename = "State")]
    state: State,

    #[serde(rename = "Balance")]
    balance: Option<Decimal>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum Type {
    Exchange,
    Transfer,
    Cashback,
    Topup,

    #[serde(rename = "CARD_PAYMENT")]
    CardPayment,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum State {
    Completed,
    Declined,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum Product {
    Current,
    Savings,
}

impl RevolutRow2023 {
    pub(crate) async fn deserialize_from(path: &PathBuf) -> Result<Vec<Trade>> {
        let now = std::time::Instant::now();
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            // .delimiter(b';')
            .delimiter(b',')
            .trim(Trim::All)
            .from_path(path)?;
        info!("ReaderBuilder::from_path done. Elapsed: {:.2?}", now.elapsed());

        let now = std::time::Instant::now();
        let mut rows: Vec<RevolutRow2023> =
            rdr.deserialize::<RevolutRow2023>()
                .filter_map(|record| record.ok())
                .collect();
        info!("reader::deserialize done. Elapsed: {:.2?}", now.elapsed());

        // 2023 Revolut csv is sorted first by Product (Current/Savings), then by date
        rows.sort_unstable_by(|a,b| a.completed_date.cmp(&b.completed_date));

        Self::rows_to_trades(&rows).await
    }

    async fn rows_to_trades(rows: &Vec<RevolutRow2023>) -> Result<Vec<Trade>> {
        let trades: Vec<Trade> =
            rows.iter()
                .fold(vec![], |mut acc, row| {
                    match row.r#type {
                        Type::Exchange | Type::CardPayment => {
                            row.to_trade()
                                .map(|trade|
                                    acc.push(trade)
                                );
                            acc
                        }
                        _ => acc
                    }
                });
        Ok(trades)
    }

    fn to_trade(&self) -> Option<Trade> {
        let mut trade = Trade::new();

        if self.amount.is_sign_positive() {
            trade.direction = Direction::Buy;
        } else  {
            trade.direction = Direction::Sell;
        }

        trade.date = self.started_date.clone();
        trade.paid_amount = self.amount;
        trade.paid_currency  = self.currency.clone();
        trade.exchanged_amount = self.fiat_amount_inc_fees.neg();
        trade.exchanged_currency = self.base_currency.clone();

        if self.product.eq(&Product::Savings) {
            trade.is_vault = true;
        }

        Some(trade)
    }
}

#[cfg(test)]
mod test {
    use crate::calculator::money::Money;
    use crate::calculator::TaxableTrade;
    use crate::calculator::trade::{Direction, Trade};
    use crate::reader::RevolutRow2023;
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
        // Current: Buy 30, Sell 30 (Trade 1), Buy 50, Transfer out 10, Transfer in 100, Sell 50 (Trade 2), Spend 25 (Trade 3) - Balance 65
        // Savings: Transfer in 10, Buy 20, Buy 40, Buy 60, Buy 80, Transfer out 100 - Balance 110
        writeln!(file, "
            Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
            EXCHANGE,Current,2023-01-01 10:00:00,2023-01-01 10:00:00,Exchanged to EOS,30.0000,EOS,600.00,609.15,9.15,SEK,COMPLETED,30.0000
            EXCHANGE,Current,2023-01-02 10:00:00,2023-01-02 10:00:00,Exchanged to SEK,-30.0000,EOS,-400.00,-394.86,5.14,SEK,COMPLETED,0.0000
            EXCHANGE,Current,2023-02-01 12:00:00,2023-02-01 12:00:00,Exchanged to EOS,50.0000,EOS,1000.00,1009.65,9.65,SEK,COMPLETED,50.0000
            TRANSFER,Current,2023-02-08 10:00:00,2023-02-08 10:00:00,Transferred to Savings,-10.0000,EOS,-200.00,-200.00,0.00,SEK,COMPLETED,40.0000
            TRANSFER,Current,2023-04-04 10:00:00,2023-04-04 10:00:00,Transferred to Current,100.0000,EOS,2000.00,2000.00,0.00,SEK,COMPLETED,140.0000
            EXCHANGE,Current,2023-04-04 11:00:00,2023-04-04 11:00:00,Exchanged to SEK,-50.0000,EOS,-600.00,-594.86,5.14,SEK,COMPLETED,90.0000
            CARD_PAYMENT,Current,2023-05-06 10:00:00,2023-05-06 10:00:00,Payment to Amazon,-25.0000,EOS,-500.00,-495.75,4.25,SEK,COMPLETED,65.0000
            TRANSFER,Savings,2023-02-08 10:00:00,2023-02-08 10:00:00,Transferred from Current,10.0000,EOS,200.00,200.00,0.00,SEK,COMPLETED,10.0000
            EXCHANGE,Savings,2023-03-01 14:00:00,2023-03-01 14:00:00,Exchanged to EOS,20.0000,EOS,400.00,404.57,4.57,SEK,COMPLETED,30.0000
            EXCHANGE,Savings,2023-03-02 14:00:00,2023-03-02 14:00:00,Exchanged to EOS,40.0000,EOS,800.00,809.15,9.15,SEK,COMPLETED,70.0000
            EXCHANGE,Savings,2023-03-03 14:00:00,2023-03-03 14:00:00,Exchanged to EOS,60.0000,EOS,1200.00,1213.73,13.73,SEK,COMPLETED,130.0000
            EXCHANGE,Savings,2023-03-04 14:00:00,2023-03-04 14:00:00,Exchanged to EOS,80.0000,EOS,1600.00,1618.31,18.31,SEK,COMPLETED,210.0000
            TRANSFER,Savings,2023-04-04 10:00:00,2023-04-04 10:00:00,Transferred to Current,-100.0000,EOS,-2000.00,-2000.00,0.00,SEK,COMPLETED,110.0000
        ")?;
        let path = file.path().to_str().unwrap();

        /*
         * When
         */
        let trades = block_on(async {
            RevolutRow2023::deserialize_from(&PathBuf::from(path)).await
        })?;

        /*
         * Then
         */
        let mut iter = trades.iter();
        assert_eq!(iter.next(), Some(&Trade {
            direction: Direction::Buy,
            paid_currency: "EOS".to_string(),
            paid_amount: dec!(30),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-609.15),
            date: "2023-01-01 10:00:00".to_string(),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(&Trade {
            direction: Direction::Sell,
            paid_currency: "EOS".to_string(),
            paid_amount: dec!(-30),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(394.86),
            date: "2023-01-02 10:00:00".to_string(),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(&Trade {
            direction: Direction::Buy,
            paid_currency: "EOS".to_string(),
            paid_amount: dec!(50),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-1009.65),
            date: "2023-02-01 12:00:00".to_string(),
            is_vault: false
        }));
        // assert_eq!(iter.next(), None);

        /*
         * When
         */
        let taxable_trades = block_on(
            TaxableTrade::taxable_trades(&trades, &"EOS".to_string(), &"SEK".to_string())
        )?;

        /*
         * Then
         */
        let mut iter = taxable_trades.into_iter();
        assert_eq!(iter.next(), Some(TaxableTrade::new(
            "2023-01-02 10:00:00".to_string(),
            "EOS".to_string(),
                dec!(-30),
            Money::new_cash("SEK".to_string(), dec!(394.86)),
            vec![Money::new_cash("SEK".to_string(), dec!(-609.15))],
            Some(dec!(-214.29))
        )));
        assert_eq!(iter.next(), Some(TaxableTrade::new(
            "2023-04-04 11:00:00".to_string(),
            "EOS".to_string(),
            dec!(-50),
            Money::new_cash("SEK".to_string(), dec!(594.86)),
            vec![Money::new_cash("SEK".to_string(), dec!(-1009.65))],
            Some(dec!(-414.79))
        )));
        assert_eq!(iter.next(), Some(TaxableTrade::new(
            "2023-05-06 10:00:00".to_string(),
            "EOS".to_string(),
            dec!(-25),
            Money::new_cash("SEK".to_string(), dec!(495.75)),
            vec![Money::new_cash("SEK".to_string(), dec!(-505.72))],
            Some(dec!(-9.97))
        )));
        assert_eq!(iter.next(), None);

        Ok(())
    }
}