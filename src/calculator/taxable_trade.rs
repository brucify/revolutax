use crate::calculator::Currency;
use crate::calculator::cost_book::CostBook;
use crate::calculator::money::Money;
use crate::calculator::trade::{Direction, Trade};
use crate::{Config, skatteverket, writer};
use anyhow::Result;
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashSet;

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug, PartialEq)]
pub(crate) struct TaxableTrade {
    date: String,
    pub(crate) currency: Currency,             // Valutakod
    pub(crate) amount: Decimal,                // Antal
    pub(crate) income: Money,                  // Försäljningspris
    pub(crate) costs: Vec<Money>,              // Omkostnadsbelopp
    net_income: Option<Decimal>,    // Vinst/förlust
}

impl Serialize for TaxableTrade {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer,
    {
        // 6 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("TaxableTrade", 6)?;
        state.serialize_field("Date", &self.date)?;
        state.serialize_field("Currency", &self.currency)?;
        state.serialize_field("Amount", &self.amount)?;
        state.serialize_field("Income", &format!("{}", self.income))?;
        state.serialize_field("Cost", &self.costs_to_string())?;
        state.serialize_field("Net Income", &self.net_income)?;
        state.end()
    }
}

impl TaxableTrade {
    pub(crate) fn new(
        date: String,
        currency: Currency,
        amount: Decimal,
        income: Money,
        costs: Vec<Money>,
        net_income: Option<Decimal>,
    ) -> Self {
        TaxableTrade {
            date,
            currency,
            amount,
            income,
            costs,
            net_income,
        }
    }

    fn costs_to_string(&self) -> String {
        if self.costs.iter().all(|c| c.is_cash()) {
            self.costs.iter()
                .fold(dec!(0), |acc, c| acc + c.amount())
                .to_string()
        } else {
            self.costs.iter()
                .fold("".to_string(), |acc, c| format!("{}, {}", acc, c))
        }
    }

    pub(crate) async fn all_taxable_trades(trades: &Vec<Trade>) -> Vec<TaxableTrade> {
        let mut unique_pairs: HashSet<(Currency, Currency)> = HashSet::new();

        for t in trades {
            let pair = (t.paid_currency.clone(), t.exchanged_currency.clone());
            unique_pairs.insert(pair);
        }

        let mut taxable_trades: Vec<TaxableTrade> = vec![];
        for pair in unique_pairs {
            let result = Self::taxable_trades(&trades, &pair.0, &pair.1).await.unwrap();
            taxable_trades.extend(result);
        }

        taxable_trades
    }

    pub(crate) async fn taxable_trades(trades: &Vec<Trade>, currency: &Currency, base_currency: &Currency) -> Result<Vec<TaxableTrade>> {
        let book = CostBook::new(currency.clone(), base_currency.clone());

        let (taxable_trades, book) =
            trades.iter()
                .fold((vec![], book), |(mut acc, mut book), trade| {
                    let currency_match =
                        trade.paid_currency.eq(&book.currency)
                            && trade.exchanged_currency.eq(&book.base_currency);
                    if currency_match {
                        match trade.direction {
                            Direction::Buy =>
                                book.add_buy(trade),
                            Direction::Sell => {
                                let taxable_trade = book.add_sell(trade).unwrap();
                                acc.push(taxable_trade);
                            }
                        }
                    }
                    (acc, book)
                });

        debug!("Remaining costs for {:?}:", book.currency);
        book.costs.iter().for_each(|c| debug!("{:?}", c));
        debug!("Taxable transactions:");
        taxable_trades.iter().for_each(|t| debug!("{:?}", t));

        Ok(taxable_trades)
    }

    pub(crate) async fn print_taxable_trades(taxable_trades: &Vec<TaxableTrade>, config: &Config) -> Result<()> {
        let taxable_trades: Vec<&TaxableTrade> =
            taxable_trades.iter()
                .filter(|t| {
                    config.year_traded
                        .map(|year_traded|
                            t.date.contains(&year_traded.to_string())
                        )
                        .unwrap_or(true)
                })
                .collect();

        if let Some(sru_conf) = &config.sru_file_config {
            Self::print_sru_file(taxable_trades, sru_conf.sru_org_num.clone(), sru_conf.sru_org_name.clone()).await?;
        } else {
            writer::print_csv_rows(&taxable_trades).await?;
        }

        Ok(())
    }

    async fn print_sru_file(taxable_trades: Vec<&TaxableTrade>, org_num: String, name: Option<String>) -> Result<()> {
        let mut res = Ok(());
        skatteverket::SruFile::try_new(taxable_trades, org_num, name)
            .map(|sru_file| {
                let stdout = std::io::stdout();
                let handle = stdout.lock();
                if let Err(e) = sru_file.write(handle) {
                    res = Err(e)
                }
            });
        res
    }
}

