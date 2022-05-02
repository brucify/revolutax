use std::io;
use crate::{calculator, reader};
use std::path::PathBuf;
use futures::executor::block_on;
use log::debug;

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges(path: &PathBuf) -> io::Result<()> {
    let now = std::time::Instant::now();
    let txns = block_on(reader::read_exchanges(path))?;
    debug!("reader::read_exchanges done. Elapsed: {:.2?}", now.elapsed());

    // txns.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_rows(&txns))?;
    debug!("reader::print_rows done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    debug!("reader::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    // rows.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_rows(&rows))?;
    debug!("reader::print_rows done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions from the path and prints the results to
/// `std::io::stdout()`.
pub fn calculate_tax(path: &PathBuf, currency: &String, base: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    debug!("reader::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    // rows.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    let txns =  block_on(reader::to_transactions(&rows, currency))?;
    debug!("reader::to_transactions done. Elapsed: {:.2?}", now.elapsed());

    // txns.iter().for_each(|t| debug!("{:?}", t));

    let now = std::time::Instant::now();
    let txns =  block_on(calculator::tax(&txns, currency, base))?;
    debug!("calculator::tax done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    block_on(calculator::print_taxables(&txns))?;
    debug!("calculator::print_taxables done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
