use anyhow::Context;
use clap::Parser;
use futures::executor::block_on;

/// Search for currency exchanges in a Revolut csv file and output a new csv containing the tax information.
#[derive(Parser)]
struct Cli {
    #[clap(parse(from_os_str), help = "Path to the Revolut transactions_history.csv file that contains transactions.")]
    path: std::path::PathBuf,

    #[clap(short, long, help = "(2022 csv only) Specify the traded cryptocurrency to report the tax for. Use 'ALL' to show all currencies when using --print-exchanges-only")]
    currency: Option<String>,

    #[clap(short, long, help = "(2022 csv only) Specify the base fiat currency to report the tax in. Defaults to 'SEK'")]
    base_currency: Option<String>,

    #[clap(long, help = "(2022 csv only) Filter the input CSV file to show only items of type 'Exchange', and print to stdout")]
    print_exchanges_only: bool,

    #[clap(long, help = "(2022 csv only) Merge two lines of a currency exchange into a single trade, and print to stdout")]
    print_trades: bool,

    #[clap(long, help = "Specify the year of the Revolut CSV file to process. Defaults to 2023")]
    csv_year: Option<u16>,
}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let csv_year: u16 = args.csv_year.unwrap_or(2023);
    let currency: String = args.currency.unwrap_or("ALL".to_string());
    let base_currency: String = args.base_currency.unwrap_or("SEK".to_string());

    match (csv_year, args.print_exchanges_only, args.print_trades) {
        (2023, _, _) => {
            block_on(cryptotax::calculate_tax_2023(&args.path))
                .with_context(|| format!("Could not calculate tax from file `{:?}`", &args.path))
                .unwrap();
        },
        (2022, true, _) => {
            match currency.as_str() {
                "ALL" =>
                    block_on(cryptotax::print_exchanges(&args.path))
                        .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                        .unwrap(),
                _ =>
                    block_on(cryptotax::print_exchanges_in_currency(&args.path, &currency))
                        .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                        .unwrap(),
            }
        },
        (2022, false, true) => {
            block_on(cryptotax::merge_exchanges(&args.path, &currency))
                .with_context(|| format!("Could not merge exchanges from file `{:?}`", &args.path))
                .unwrap();
        },
        (2022, false, false) => {
            block_on(cryptotax::calculate_tax_2022(&args.path, &currency, &base_currency))
                .with_context(|| format!("Could not calculate tax from file `{:?}`", &args.path))
                .unwrap();
        },
        _ => {
            println!("Unknown csv year")
        },
    }
}
