use std::io;
use crate::{calculator, reader};
use std::path::PathBuf;
use futures::executor::block_on;
use log::{debug, info};

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges(path: &PathBuf) -> io::Result<()> {
    let now = std::time::Instant::now();
    let txns = block_on(reader::read_exchanges(path))?;
    debug!("deserialize_from_path done. Elapsed: {:.2?}", now.elapsed());

    txns.iter().for_each(|t| info!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_rows(&txns))?;
    debug!("print_transactions done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    debug!("deserialize_from_path done. Elapsed: {:.2?}", now.elapsed());

    rows.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_rows(&rows))?;
    debug!("print_transactions done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions from the path and prints the results to
/// `std::io::stdout()`.
pub fn calculate_tax(path: &PathBuf, currency: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    debug!("deserialize_from_path done. Elapsed: {:.2?}", now.elapsed());

    rows.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    let txns =  block_on(reader::to_transactions(&rows, currency))?;

    txns.iter().for_each(|t| debug!("{:?}", t));

    let _ =  block_on(calculator::tax(&txns, currency, &"SEK".to_string()))?;

    block_on(reader::print_rows(&rows))?;
    debug!("print_transactions done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
