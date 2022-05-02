use crate::calculator::TaxableTransaction;
use crate::transaction::Currency;
use csv::WriterBuilder;
use rust_decimal::Decimal;
use serde::Serialize;
use std::io;

#[derive(Debug, Serialize)]
pub(crate) struct Row {
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Currency")]
    currency: Currency,             // Valutakod
    #[serde(rename = "Amount")]
    amount: Decimal,                // Antal
    #[serde(rename = "Income")]
    income: String,                 // Försäljningspris
    #[serde(rename = "Cost")]
    cost: String,                   // Omkostnadsbelopp
    #[serde(rename = "Net Income")]
    net_income: Option<Decimal>,    // Vinst/förlust
}

impl Row {
    pub(crate) fn new(date: String,
                      currency: Currency,
                      amount: Decimal,
                      income: String,
                      cost: String,
                      net_income: Option<Decimal>) -> Row {
        Row { date, currency, amount, income, cost, net_income }
    }
}

/// Wraps the `stdout.lock()` in a `csv::Writer` and writes the rows.
/// The `csv::Writer` is already buffered so there is no need to wrap
/// `stdout.lock()` in a `io::BufWriter`.
pub(crate) async fn print_taxables(txns: &Vec<TaxableTransaction>) -> io::Result<()> {
    let rows: Vec<Row> = txns.iter().map(|t| t.to_row()).collect();
    print(&rows).await
}

/// Wraps the `stdout.lock()` in a `csv::Writer` and writes the rows.
/// The `csv::Writer` is already buffered so there is no need to wrap
/// `stdout.lock()` in a `io::BufWriter`.
pub(crate) async fn print<S: Serialize>(rows: &Vec<S>) -> io::Result<()>{
    let stdout = io::stdout();
    let lock = stdout.lock();
    let mut wtr =
        WriterBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_writer(lock);

    let mut err = None;
    rows.iter().for_each(|row|
        wtr.serialize(row)
            .unwrap_or_else(|e| {
                err = Some(e);
                Default::default()
            })
    );
    err.map_or(Ok(()), Err)?;
    Ok(())
}