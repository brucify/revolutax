use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::executor::block_on;
use log::error;

/// Search for currency exchanges in a Revolut csv file and output a new csv containing the tax information.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(help = "Path to the Revolut transactions_history.csv file that contains transactions.")]
    path: std::path::PathBuf,

    #[arg(short, long, help = "(2022 csv only) Specify the traded cryptocurrency to report the tax for. Use 'ALL' to show all currencies when using --print-exchanges-only")]
    currency: Option<String>,

    #[arg(short, long, help = "(2022 csv only) Specify the base fiat currency to report the tax in. Defaults to 'SEK'")]
    base_currency: Option<String>,

    #[arg(long, help = "(2022 csv only) Filter the input CSV file to show only items of type 'Exchange', and print to stdout")]
    print_exchanges_only: bool,

    #[arg(long, help = "(2022 csv only) Merge two lines of a currency exchange into a single trade, and print to stdout")]
    print_trades: bool,

    #[arg(long, help = "Print taxable trades in the Swedish Tax Agency's SRU file format")]
    sru_file: bool,

    #[arg(long, help = "Personal/organisational number to print in the SRU file")]
    sru_org_num: Option<String>,

    #[arg(long, help = "Name to print in the SRU file")]
    sru_org_name: Option<String>,

    #[arg(long, help = "In the SRU file summarize taxable trades by currency")]
    sru_sum: bool,

    #[arg(long, help = "Only include taxable trades from this year")]
    year_traded: Option<u16>,

    #[arg(long, help = "Specify the year of the Revolut CSV file to process. Defaults to 2023")]
    csv_version: Option<u16>,
}

impl Cli {
    fn to_config(self) -> Result<cryptotax::Config> {
        let Cli {
            path,
            currency,
            base_currency,
            print_exchanges_only,
            print_trades,
            sru_file,
            sru_org_num,
            sru_org_name,
            sru_sum,
            year_traded,
            csv_version,
        } = self;

        let sru_file_config = if sru_file {
            Some(cryptotax::SruFileConfig {
                sru_org_num: sru_org_num.ok_or(anyhow!("--sru_org_num <SRU_ORG_NUM> is mandatory if --sru_file is given"))?,
                sru_org_name,
                sru_sum,
            })
        } else {
            None
        };

        let config = cryptotax::Config {
            path,
            currency: currency.unwrap_or("ALL".to_string()),
            base_currency: base_currency.unwrap_or("SEK".to_string()),
            print_exchanges_only,
            print_trades,
            sru_file_config,
            year_traded,
            csv_version: csv_version.unwrap_or(2023),
        };

        Ok(config)
    }
}


fn main() {
    env_logger::init();
    let args = Cli::parse();
    let config = args.to_config().with_context(|| format!("Invalid command line flags")).unwrap();

    match (config.csv_version, config.print_exchanges_only, config.print_trades) {
        (2022, true, _) => {
            match config.currency.as_str() {
                "ALL" => block_on(cryptotax::print_exchanges(&config.path)),
                _ => block_on(cryptotax::print_exchanges_in_currency(&config.path, &config.currency)),
            }
                .with_context(|| format!("Could not read transactions from file `{:?}`", &config.path))
                .unwrap();
        },
        (2022, false, true) => {
            block_on(cryptotax::merge_exchanges(&config.path, &config.currency))
                .with_context(|| format!("Could not merge exchanges from file `{:?}`", &config.path))
                .unwrap();
        },
        (2022, false, false) => {
            block_on(cryptotax::calculate_tax_v2022(&config))
                .with_context(|| format!("Could not calculate tax from file `{:?}`", &config.path))
                .unwrap();
        },
        (2023, _, _) => {
            block_on(cryptotax::calculate_tax_v2023(&config))
                .with_context(|| format!("Could not calculate tax from file `{:?}`", &config.path))
                .unwrap();
        },
        _ => {
            error!("Unknown csv year")
        },
    }
}
