pub(crate) mod revolut_row;

use crate::calculator::Currency;
use crate::reader::revolut_row::{RevolutRow, State, Type};
use csv::{ReaderBuilder, Trim};
use log::info;
use std::path::PathBuf;

/// Reads the file from path into a `Vec<Row>`.
async fn deserialize_from(path: &PathBuf) -> std::io::Result<Vec<RevolutRow>> {
    let now = std::time::Instant::now();
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        // .delimiter(b';')
        .delimiter(b',')
        .trim(Trim::All)
        .from_path(path)?;
    info!("ReaderBuilder::from_path done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let rows: Vec<RevolutRow> =
        rdr.deserialize::<RevolutRow>()
            .filter_map(|record| record.ok())
            .collect();
    info!("reader::deserialize done. Elapsed: {:.2?}", now.elapsed());

    Ok(rows)
}

/// Reads the file from path into a `Vec<Row>`, returns only rows with type `Exchange`.
pub(crate) async fn read_exchanges(path: &PathBuf) -> std::io::Result<Vec<RevolutRow>> {
    let rows = deserialize_from(path).await?
        .into_iter()
        .filter(|t| t.r#type == Type::Exchange)
        .collect();
    Ok(rows)
}

/// Reads the file from path into a `Vec<Row>`, returns only rows with type `Exchange` in the
/// target currency, or  with type `Card Payment` but in the target currency.
pub(crate) async fn read_exchanges_in_currency(path: &PathBuf, currency: &Currency) -> std::io::Result<Vec<RevolutRow>> {
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