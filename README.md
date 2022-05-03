cryptotax
=====

A Rust CLI application. Searches for currency exchanges in a Revolut CSV file and outputs a new csv containing the tax
information.

```shell
cryptotax
Search for currency exchanges in a Revolut csv file and output a new csv containing the tax
information

USAGE:
    cryptotax [OPTIONS] <PATH>

ARGS:
    <PATH>    Path to the Revolut transactions_history.csv file that contains transactions.

OPTIONS:
    -b, --base <BASE>            Base currency. The currency in which you report the tax. Default:
                                 'SEK'
    -c, --currency <CURRENCY>    The traded currency for which you report the tax. 'ALL' for all
                                 currencies when --exchanges is used
    -e, --exchanges              Filter the input csv file. Print to stdout a new csv file with
                                 items with type 'Exchange' only
    -h, --help                   Print help information
    -t, --transactions           Merge both sides of a currency 'Exchange' into a single line. Print
                                 to stdout a new csv file
```

Examples
-----

You will typically receive a `transaction_history.csv` file from Revolut customer service which looks like this:


| Type         | Started Date        | Completed Date      | Description        | Amount      | Fee        | Currency | Original Amount | Original Currency | Settled Amount | Settled Currency | State     | Balance    |
|--------------|---------------------|---------------------|--------------------|-------------|------------|----------|-----------------|-------------------|----------------|------------------|-----------|------------|
| Exchange     | 2022-05-02 08:00:00 | 2022-05-02 08:00:00 | Exchanged to BTC   | -100.00     | -1.00      | SEK      | -100.00         | SEK               |                |                  | Completed | 200.00     |
| Exchange     | 2022-05-02 08:00:00 | 2022-05-02 08:00:00 | Exchanged from SEK | 0.00010000  | 0.00000000 | BTC      | 0.00010000      | BTC               |                |                  | Completed | 0.00010000 |
| Card Payment | 2022-04-01 17:00:00 | 2020-04-06 03:00:00 | Klarna             | -0.00100000 | 0.00000000 | BTC      | -500.00         | SEK               | 500.00         | SEK              | Completed | 0.00000000 |

The program reads the transactions of type `Exchange` and `Card Payment` and generates a new csv file `tax_btc.csv`:

```bash
$ cargo run -- transactions_history.csv --currency ETH --base SEK > tax_btc.csv
```
| Date                | Currency | Amount | Income                       | Cost                                                                   | Net Income |
|---------------------|----------|--------|------------------------------|------------------------------------------------------------------------|------------|
| 2022-05-02 17:00:00 | BTC      | -0.005 | 500                          | -600                                                                   | -100       |
| 2022-04-01 22:00:00 | BTC      | -0.02  | 2000                         | -1500                                                                  | 500        |
| 2021-12-01 22:00:00 | BTC      | -0.3   | 30000                        | -25000                                                                 | 5000       |
| 2021-11-09 16:00:00 | BTC      | -0.001 | (20 EOS 2021-11-09 16:00:00) | -300                                                                   |            |
| 2021-11-01 09:00:00 | BTC      | -0.001 | (50 DOT 2021-11-01 09:00:00) | (-150 USD 2021-10-03 07:00:00), (-100 DOGE 2021-10-02 17:30:00), -3000 |            |
| 2021-11-01 08:30:00 | BTC      | -0.001 | 20000                        | (-100000 DOGE 2021-10-02 17:30:00)                                     |            |


Or just outputs the trades in a new csv file `txns_btc.csv`:

```bash
$ cargo run -- transactions_history.csv --currency ETH --base SEK --transactions > txn_btc.csv
```

| Type | Paid Currency | Paid Amount | Exchanged Currency | Exchanged Amount | Date                | Vault |
|------|---------------|-------------|--------------------|------------------|---------------------|-------|
| Buy  | BTC           | 0.00003000  | SEK                | -2               | 2022-05-01 06:00:00 | false |
| Buy  | BTC           | 0.00006000  | SEK                | -3.82            | 2022-05-01 10:00:00 | false |
| Buy  | BTC           | 0.00006667  | SEK                | -4.1             | 2022-05-01 19:30:00 | false |
| Buy  | BTC           | 0.00005000  | SEK                | -3               | 2022-05-01 23:30:00 | false |
| Sell | BTC           | -0.00005000 | SEK                | -3               | 2022-05-01 23:30:00 | false |


Build
-----

    $ cargo build

Run
-----

    $ cargo run -- transactions_history.csv --currency ETH --base SEK > tax_eth.csv
    $ cargo run -- transactions_history.csv --currency ETH --exchanges > exchanges_eth.csv
    $ cargo run -- transactions_history.csv --currency ETH --transactions > txns_eth.csv

Or

    $ cargo build
    $ target/debug/cryptotax transactions.csv --currency ETH > eth.csv

Optionally, set the environment variable`RUST_LOG` to `info` or `debug` to see more logs.

    $ RUST_LOG=debug cargo run -- transactions_history.csv --currency ETH > eth.csv
