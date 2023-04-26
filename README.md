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

    $ cargo run -- account_statement.csv > tax.csv

To see more logs, set the environment variable RUST_LOG to info or debug:

    $ RUST_LOG=debug cargo run -- account_statement.csv > tax.csv

Here is an example input CSV file `account_statement.csv`:

```csv
Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
EXCHANGE,Current,2023-01-01 10:00:00,2023-01-01 10:00:00,Exchanged to EOS,100.0000,EOS,600.00,609.15,9.15,SEK,COMPLETED,100.0000
EXCHANGE,Savings,2023-01-01 11:00:00,2023-01-01 11:00:00,Exchanged to EOS,50.0000,EOS,300.00,304.15,4.15,SEK,COMPLETED,50.0000
EXCHANGE,Current,2023-01-02 10:00:00,2023-01-02 10:00:00,Exchanged to SEK,-30.0000,EOS,-400.00,-394.86,5.14,SEK,COMPLETED,70.0000
CARD_PAYMENT,Current,2023-05-06 10:00:00,2023-05-06 10:00:00,Payment to Amazon,-25.0000,EOS,-500.00,-495.75,4.25,SEK,COMPLETED,45.0000
```

The program reads the input CSV file and outputs the following tax report `tax.csv`:

```csv
Date;Currency;Amount;Income;Cost;Net Income
2023-01-02 10:00:00;EOS;-30;394.86;-182.7450;212.1150
2023-05-06 10:00:00;EOS;-25;495.75;-152.2875;343.4625
```
This report shows the following information for each transaction:

* Date: The date and time when the transaction occurred.
* Currency (`Valutakod`): The currency used in the transaction.
* Amount (`Antal`): The amount of currency used in the transaction.
* Income (`Försäljningspris`): The income generated from the transaction, calculated in the base currency (SEK).
* Cost (`Omkostnadsbelopp`): The cost of the transaction, calculated in the base currency (SEK).
* Net Income (`Vinst/förlust`): The net income generated from the transaction, calculated by subtracting the cost from the income.
  
#### Current vs. Savings

The program algorithm  takes into account the two types of `Product` of transactions: `Savings` and `Current`.
Specifically, when calculating the tax for a trade where you sold cryptocurrency, the program will first try to
find the costs for the sold crypto in the `Current` transactions and deduct them from there. Only when there are
not enough available costs to deduct on the `Current` transactions will the program deduct from the `Savings`
transactions.

## License
```
This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
```