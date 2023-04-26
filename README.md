# cryptotax

A Rust CLI tool for calculating taxes on cryptocurrencies traded on Revolut. It reads the 
account statement CSV file from Revolut and generates a new CSV file summarizing the taxable
trades.

Although this program is specifically designed for reporting Swedish taxes to Skatteverket,
it can also be used for general tax reporting purposes in other countries.

## Installation

To install Rust, follow the official installation guide available at [rustup.rs](https://rustup.rs/).

After Rust is installed, clone the repository and build the project:


    $ cargo build

## Usage

The program reads the transactions of type `EXCHANGE` and `CARD_PAYMENT` and generates a new csv file tax.csv. For example:

    $ cargo run -- account_statement.csv --base-currency SEK > tax.csv

To see more logs, set the environment variable RUST_LOG to info or debug:

    $ RUST_LOG=debug cargo run -- account_statement.csv --base-currency SEK > tax.csv

Here is an example input CSV file `account_statement.csv`:

```csv
Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
EXCHANGE,Current,2023-01-01 10:00:00,2023-01-01 10:00:00,Exchanged to EOS,30.0000,EOS,600.00,609.15,9.15,SEK,COMPLETED,30.0000
EXCHANGE,Current,2023-01-02 10:00:00,2023-01-02 10:00:00,Exchanged to SEK,-30.0000,EOS,-400.00,-394.86,5.14,SEK,COMPLETED,0.0000
TRANSFER,Current,2023-02-08 10:00:00,2023-02-08 10:00:00,Transferred to Savings,-10.0000,EOS,-200.00,-200.00,0.00,SEK,COMPLETED,40.0000
TRANSFER,Current,2023-04-04 10:00:00,2023-04-04 10:00:00,Transferred to Current,100.0000,EOS,2000.00,2000.00,0.00,SEK,COMPLETED,140.0000
CARD_PAYMENT,Current,2023-05-06 10:00:00,2023-05-06 10:00:00,Payment to Amazon,-25.0000,EOS,-500.00,-495.75,4.25,SEK,COMPLETED,65.0000
```

The program reads the input CSV file and outputs the following tax report `tax.csv`:

```csv
Date;Currency;Amount;Income;Cost;Net Income
2023-01-02 10:00:00;EOS;-30;394.86;-609.1500;-214.2900
2023-04-04 11:00:00;EOS;-50;594.86;-1009.6500;-414.7900
2023-05-06 10:00:00;EOS;-25;495.75;-505.7200;-9.9700
```
This report shows the following information for each transaction:

* Date: The date and time when the transaction occurred.
* Currency (`Valutakod`): The currency used in the transaction.
* Amount (`Antal`): The amount of currency used in the transaction.
* Income (`Försäljningspris`): The income generated from the transaction, calculated in the base currency (SEK).
* Cost (`Omkostnadsbelopp`): The cost of the transaction, calculated in the base currency (SEK).
* Net Income (`Vinst/förlust`): The net income generated from the transaction, calculated by subtracting the cost from the income.