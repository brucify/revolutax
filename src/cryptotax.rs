use crate::{calculator, reader, writer};
use futures::executor::block_on;
use log::info;
use std::io;
use std::path::PathBuf;

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub fn print_exchanges(path: &PathBuf) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges(path))?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    block_on(writer::print(&rows))?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// and finally prints the results to `std::io::stdout()`.
pub fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    block_on(writer::print(&rows))?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// and finally prints the results to `std::io::stdout()`.
pub fn merge_exchanges(path: &PathBuf, currency: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    info!("reader::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let txns =  block_on(reader::to_transactions(&rows, currency))?;
    info!("reader::to_transactions done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    block_on(writer::print(&txns))?;
    info!("calculator::tax done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// calculates tax from the transactions,
/// and finally prints the results to `std::io::stdout()`.
pub fn calculate_tax(path: &PathBuf, currency: &String, base: &String) -> io::Result<()> {
    let now = std::time::Instant::now();
    let rows = block_on(reader::read_exchanges_in_currency(path, currency))?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let txns =  block_on(reader::to_transactions(&rows, currency))?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let txns =  block_on(calculator::tax(&txns, currency, base))?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    block_on(writer::print(&txns))?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
