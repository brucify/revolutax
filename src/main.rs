use anyhow::Context;
use cryptotax::cryptotax;
use clap::Parser;

/// Search for currency exchanges in a Revolut csv file and output a new csv containing the tax information.
#[derive(Parser)]
struct Cli {
    #[clap(parse(from_os_str), help = "Path to the Revolut transactions_history.csv file that contains transactions.")]
    path: std::path::PathBuf,

    #[clap(short, long, help = "The traded currency for which you report the tax. 'ALL' for all currencies when --exchanges is used")]
    currency: Option<String>,

    #[clap(short, long, help = "Base currency. The currency in which you report the tax. Default: 'SEK'")]
    base: Option<String>,

    #[clap(short, long, help = "Filter the input csv file. Print to stdout a new csv file with items with type 'Exchange' only")]
    exchanges: bool,

    #[clap(short, long, help = "Merge two lines of a currency 'Exchange' into a single trade. Print to stdout a new csv file")]
    trades: bool,

}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let currency: String = args.currency.unwrap_or("ALL".to_string());
    let base: String = args.base.unwrap_or("SEK".to_string());

    if args.exchanges {
        match currency.as_str() {
            "ALL" => cryptotax::print_exchanges(&args.path)
                .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                .unwrap(),
            _ => cryptotax::print_exchanges_in_currency(&args.path, &currency)
                .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                .unwrap(),
        }
    } else if args.trades {
        cryptotax::merge_exchanges(&args.path, &currency)
            .with_context(|| format!("Could not merge exchanges from file `{:?}`", &args.path))
            .unwrap();
    } else {
        cryptotax::calculate_tax(&args.path, &currency, &base)
            .with_context(|| format!("Could not calculate tax from file `{:?}`", &args.path))
            .unwrap();
    }
}
