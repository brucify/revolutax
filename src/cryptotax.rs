use crate::{calculator, reader, writer};
use log::info;
use std::path::PathBuf;

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub async fn print_exchanges(path: &PathBuf) -> std::io::Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::read_exchanges(path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&rows).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// and finally prints the results to `std::io::stdout()`.
pub async fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> std::io::Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::read_exchanges_in_currency(path, currency).await?;
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
pub async fn merge_exchanges(path: &PathBuf, currency: &String) -> std::io::Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::read_exchanges_in_currency(path, currency).await?;
    info!("reader::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let trades =  reader::revolut_row::to_trades(&rows, currency).await?;
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
pub async fn calculate_tax(path: &PathBuf, currency: &String, base: &String) -> std::io::Result<()> {
    let now = std::time::Instant::now();
    let rows = reader::read_exchanges_in_currency(path, currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let trades = reader::revolut_row::to_trades(&rows, currency).await?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    let trades = calculator::tax(&trades, currency, base).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = std::time::Instant::now();
    writer::print(&trades).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
