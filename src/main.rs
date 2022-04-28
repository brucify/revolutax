use anyhow::Context;
use cryptotax::cryptotax;
use clap::Parser;

/// Search for currency exchanges in a Revolut csv file and output a new csv containing the tax information.
#[derive(Parser)]
struct Cli {
    #[clap(parse(from_os_str), help = "Path to the csv file that contains transactions.")]
    path: std::path::PathBuf,

    #[clap(short, long, help = "Give 'ALL' for all currencies.")]
    currency: Option<String>,

    #[clap(short, long, help = "Print to stdout a new csv file with transactions with type 'Exchange'")]
    exchanges: bool,

}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let currency: String = args.currency.unwrap_or("ALL".to_string());

    if args.exchanges {
        match currency.as_str() {
            "ALL" => cryptotax::print_exchanges(&args.path)
                .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                .unwrap(),
            _ => cryptotax::print_exchanges_in_currency(&args.path, &currency)
                .with_context(|| format!("Could not read transactions from file `{:?}`", &args.path))
                .unwrap(),
        }
    } else {
        cryptotax::calculate_tax(&args.path, &currency)
            .with_context(|| format!("Could not calculate tax from file `{:?}`", &args.path))
            .unwrap();
    }
}
