use crate::{calculator, reader, writer};
use log::info;
use std::io::Result;
use std::path::PathBuf;

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub async fn print_exchanges(path: &PathBuf) -> Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::RevolutRow2022::read_exchanges(path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&rows).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// and finally prints the results to `std::io::stdout()`.
pub async fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&rows).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// and finally prints the results to `std::io::stdout()`.
pub async fn merge_exchanges(path: &PathBuf, currency: &String) -> Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("reader::RevolutRow2022::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let trades =  reader::RevolutRow2022::rows_to_trades(&rows, currency).await?;
    info!("reader::to_transactions done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&trades).await?;
    info!("calculator::tax done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// calculates tax from the transactions,
/// and finally prints the results to `std::io::stdout()`.
pub async fn calculate_tax_2022(path: &PathBuf, currency: &String, base: &String) -> Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let trades = reader::RevolutRow2022::rows_to_trades(&rows, currency).await?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let taxable_trades = calculator::taxable_trades(&trades, currency, base).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&taxable_trades).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

pub async fn calculate_tax_2023(path: &PathBuf, currency: &String, base: &String) -> Result<()> {
    let now = std::time::Instant::now();
    let trades = reader::RevolutRow2023::deserialize_from(path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let taxable_trades = calculator::taxable_trades(&trades, currency, base).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&taxable_trades).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
