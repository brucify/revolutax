use anyhow::{anyhow, Result};
use log::info;
use reader::RevolutRow2022;
use reader::RevolutRow2023;
use std::path::PathBuf;
use std::time::Instant;
use crate::calculator::TaxableTrade;

mod calculator;
mod reader;
mod writer;
mod skatteverket;

pub struct Config {
    pub path: PathBuf,
    pub currency: String,
    pub base_currency: String,
    pub print_exchanges_only: bool,
    pub print_trades: bool,
    pub sru_file_config: Option<SruFileConfig>,
    pub csv_year: u16,
}

pub struct SruFileConfig {
    pub sru_org_num: String,
    pub sru_name: Option<String>,
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
pub async fn calculate_tax_2022(config: &Config) -> Result<()> {
    let now = Instant::now();
    let rows = RevolutRow2022::read_exchanges_in_currency(&config.path, &config.currency).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let trades = RevolutRow2022::rows_to_trades(&rows, &config.currency).await?;
    info!("Done converting to transactions. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let taxable_trades = calculator::taxable_trades(&trades, &config.currency, &config.base_currency).await?;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    print_result(&taxable_trades, config).await?;
    info!("Done printing results. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

pub async fn calculate_tax_2023(config: &Config) -> Result<()> {
    let now = Instant::now();
    let trades = RevolutRow2023::deserialize_from(&config.path).await?;
    info!("Done reading csv file. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    let taxable_trades = calculator::all_taxable_trades(&trades).await;
    info!("Done calculating taxes. Elapsed: {:.2?}", now.elapsed());

    let now = Instant::now();
    print_result(&taxable_trades, config).await?;
    info!("Done printing results. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}

async fn print_result(taxable_trades: &Vec<TaxableTrade>, config: &Config) -> Result<()> {
    if let Some(sru_conf) = &config.sru_file_config {
        print_sru_file(&taxable_trades, sru_conf.sru_org_num.clone(), sru_conf.sru_name.clone()).await?;
    } else {
        writer::print_csv_rows(&taxable_trades).await?;
    }
    Ok(())
}

async fn print_sru_file(taxable_trades: &Vec<TaxableTrade>, org_num: String, name: Option<String>) -> Result<()> {
    let now = Instant::now();
    let sru_file = skatteverket::SruFile::try_new(taxable_trades, org_num, name)
        .ok_or(anyhow!("Failed to create SRU file from taxable trades"))?;
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    sru_file.write(handle)?;
    info!("Done printing SRU file. Elapsed: {:.2?}", now.elapsed());

    Ok(())
}
