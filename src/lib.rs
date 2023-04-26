use log::info;
use reader::RevolutRow2022;
use reader::RevolutRow2023;
use std::io::Result;
use std::path::PathBuf;
use std::time::Instant;

mod calculator;
mod reader;
mod writer;

/// Reads the transactions with type `Exchange` from the path and prints the results to
/// `std::io::stdout()`.
pub async fn print_exchanges(path: &PathBuf) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges(path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    writer::print_csv_rows(&rows).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// and finally prints the results to `std::io::stdout()`.
pub async fn print_exchanges_in_currency(path: &PathBuf, currency: &String) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    writer::print_csv_rows(&rows).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// and finally prints the results to `std::io::stdout()`.
pub async fn merge_exchanges(path: &PathBuf, currency: &String) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("RevolutRow2022::read_exchanges_in_currency done. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let trades =  RevolutRow2022::rows_to_trades(&rows, currency).await?;
    info!("reader::to_transactions done. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    writer::print_csv_rows(&trades).await?;
    info!("calculator::tax done. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

/// Reads the transactions with type `Exchange` from the path,
/// filters for the target currency,
/// converts the csv rows into transactions,
/// calculates tax from the transactions,
/// and finally prints the results to `std::io::stdout()`.
pub async fn calculate_tax_2022(path: &PathBuf, currency: &String, base_currency: &String) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges_in_currency(path, currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let trades = RevolutRow2022::rows_to_trades(&rows, currency).await?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let taxable_trades = calculator::taxable_trades(&trades, currency, base_currency).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    writer::print_csv_rows(&taxable_trades).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

pub async fn calculate_tax_2023(path: &PathBuf) -> Result<()> {
    let now = Instant::now();
    let trades = RevolutRow2023::deserialize_from(path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let currency = trades.first().map(|t| t.paid_currency.clone()).unwrap();
    let base_currency = trades.first().map(|t| t.exchanged_currency.clone()).unwrap();
    let taxable_trades = calculator::taxable_trades(&trades, &currency, &base_currency).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    writer::print_csv_rows(&taxable_trades).await?;
    info!("Done printing rows. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
