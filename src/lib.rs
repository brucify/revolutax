use anyhow::Result;
use log::info;
use std::path::PathBuf;
use std::time::Instant;

mod calculator;
mod reader;
mod writer;
mod skatteverket;

use self::calculator::TaxableTrade;
use self::reader::{RevolutRow2022, RevolutRow2023};

pub struct Config {
    pub path: PathBuf,
    pub currency: String,
    pub base_currency: String,
    pub print_exchanges_only: bool,
    pub print_trades: bool,
    pub sru_file_config: Option<SruFileConfig>,
    pub year_traded: Option<u16>,
    pub sum: bool,
    pub csv_version: u16,
}

pub struct SruFileConfig {
    pub sru_org_num: String,
    pub sru_org_name: Option<String>,
}

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
pub async fn calculate_tax_v2022(config: &Config) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges_in_currency(&config.path, &config.currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let trades = RevolutRow2022::rows_to_trades(&rows, &config.currency).await?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let taxable_trades =
        TaxableTrade::taxable_trades(
            &trades,
            &config.currency,
            &config.base_currency
        ).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    TaxableTrade::print_taxable_trades(taxable_trades, config).await?;
    info!("Done printing results. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

pub async fn calculate_tax_v2023(config: &Config) -> Result<()> {
    let now = Instant::now();
    let trades = RevolutRow2023::deserialize_from(&config.path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let taxable_trades = TaxableTrade::taxable_trades_all_currencies(&trades).await;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    TaxableTrade::print_taxable_trades(taxable_trades, config).await?;
    info!("Done printing results. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
