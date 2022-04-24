use std::io;
use crate::reader;
use std::path::PathBuf;
use futures::executor::block_on;
use log::{debug, info};

/// Reads the transactions from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges(path: &PathBuf) -> io::Result<()> {
    let now = std::time::Instant::now();
    let txns = block_on(reader::read_exchanges(path))?;
    debug!("deserialize_from_path done. Elapsed: {:.2?}", now.elapsed());

    txns.iter().for_each(|t| info!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_transactions(&txns))?;
    debug!("print_transactions done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges_in_currency(path: &PathBuf, currency: String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let txns = block_on(reader::read_exchanges_in_currency(path, currency))?;
    debug!("deserialize_from_path done. Elapsed: {:.2?}", now.elapsed());

    txns.iter().for_each(|t| info!("{:?}", t));

    let now = std::time::Instant::now();
    block_on(reader::print_transactions(&txns))?;
    debug!("print_transactions done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
