use anyhow::{anyhow, Result};
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::{HashMap, HashSet};

use super::{CostBook, Currency, Direction, Trade, Money};
use crate::{Config, writer};
use crate::skatteverket::SruFile;

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug, PartialEq)]
pub(crate) struct TaxableTrade {
    date: Option<String>,
    pub(crate) currency: Currency,             // Valutakod
    pub(crate) amount: Decimal,                // Antal
    pub(crate) income: Money,                  // Försäljningspris
    pub(crate) costs: Vec<Money>,              // Omkostnadsbelopp
    pub(crate) net_income: Option<Decimal>,    // Vinst/förlust
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
        date: Option<String>,
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
        if let Some(sum) = self.sum_cash_amount() {
            sum.to_string()
        } else {
            self.costs.iter()
                .fold("".to_string(), |acc, c| format!("{}, {}", acc, c))
        }
    }

    pub(crate) fn sum_cash_amount(&self) -> Option<Decimal> {
        if self.costs.iter().all(|c| c.is_cash()) {
            let sum =
                self.costs.iter()
                    .fold(dec!(0), |acc, c| acc + c.amount());
            Some(sum)
        } else {
            None
        }
    }

    pub(crate) async fn taxable_trades_all_currencies(trades: &Vec<Trade>) -> Vec<TaxableTrade> {
        let mut unique_pairs: HashSet<(Currency, Currency)> = HashSet::new();

        for t in trades {
            let pair = (t.paid_currency.clone(), t.exchanged_currency.clone());
            unique_pairs.insert(pair);
        }

        let mut taxable_trades: Vec<TaxableTrade> = vec![];

        for (paid_currency, exchanged_currency) in unique_pairs {
            let result =
                Self::taxable_trades(
                    &trades,
                    &paid_currency,
                    &exchanged_currency
                ).await.unwrap();
            taxable_trades.extend(result);
        }

        taxable_trades
    }

    pub(crate) async fn taxable_trades(
        trades: &Vec<Trade>,
        currency: &Currency,
        base_currency: &Currency
    ) -> Result<Vec<TaxableTrade>> {
        let book = CostBook::new(currency.clone(), base_currency.clone());

        let mut err = Ok(());

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
                                match book.add_sell(trade) {
                                    Ok(taxable_trade) => acc.push(taxable_trade),
                                    Err(e) => err = Err(e),
                                }
                            }
                        }
                    }

                    (acc, book)
                });

        err?;

        debug!("Remaining costs for {:?}:", book.currency);
        book.costs.iter().for_each(|c| debug!("{:?}", c));
        debug!("Taxable transactions:");
        taxable_trades.iter().for_each(|t| debug!("{:?}", t));

        Ok(taxable_trades)
    }

    pub(crate) async fn print_taxable_trades(
        taxable_trades: Vec<TaxableTrade>,
        config: &Config
    ) -> Result<()> {
        let taxable_trades =
            taxable_trades.into_iter()
                .filter(|t| {
                    config.year_traded
                        .map(|year_traded|
                            t.date.as_ref().map(|date|
                                date.contains(&year_traded.to_string())
                            )
                        )
                        .flatten()
                        .unwrap_or(true)
                })
                .collect();

        let taxable_trades =
            if config.sum {
                TaxableTrade::try_sum_cash_amount_by_currency(&taxable_trades)?
            } else {
                taxable_trades
            };

        if let Some(sru_conf) = &config.sru_file_config {
            Self::print_sru_file(
                &taxable_trades,
                sru_conf.sru_org_num.clone(),
                sru_conf.sru_org_name.clone()
            ).await?;
        } else {
            writer::print_csv_rows(&taxable_trades).await?;
        }

        Ok(())
    }

    async fn print_sru_file(
        taxable_trades: &Vec<TaxableTrade>,
        org_num: String,
        name: Option<String>
    ) -> Result<()> {
        let mut res = Ok(());
        SruFile::try_new(taxable_trades, org_num, name)
            .map(|sru_file| {
                let stdout = std::io::stdout();
                let handle = stdout.lock();
                if let Err(e) = sru_file.write(handle) {
                    res = Err(e)
                }
            });
        res
    }

    pub(crate) fn try_sum_cash_amount_by_currency(taxable_trades: &Vec<TaxableTrade>) -> Result<Vec<TaxableTrade>> {
        let mut summary_map: HashMap<Currency, (Decimal, Decimal, Decimal)> = HashMap::new();

        let mut err = Ok(());

        for trade in taxable_trades {
            if let Some(costs) = trade.sum_cash_amount() {
                let (acc_amount, acc_income, acc_costs) =
                    summary_map.entry(trade.currency.clone())
                        .or_insert((dec!(0), dec!(0), dec!(0)));
                *acc_amount += trade.amount;
                *acc_income += trade.income.amount();
                *acc_costs += costs;
            } else {
                err = Err(anyhow!("All costs must be cash"));
            }
        }

        err?;

        let sum =
            summary_map.into_iter()
                .map(|(currency, (amount, income, costs))|
                    TaxableTrade::new(
                        None,
                        currency,
                        amount,
                        Money::new_cash("UNKNOWN".to_string(), income),
                        vec![Money::new_cash("UNKNOWN".to_string(), costs)],
                        Some(income + costs)
                    )
                )
                .collect();

        Ok(sum)
    }
}

