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
    -e, --exchanges              Print to stdout a new csv file with transactions with type
                                 'Exchange' only
    -h, --help                   Print help information
```


Build
-----

    $ cargo build

Run
-----

    $ cargo run -- transactions_history.csv --currency ETH --base SEK > tax_eth.csv
    $ cargo run -- transactions_history.csv --currency ETH --exchanges > eth_exchanges.csv

Or

    $ cargo build
    $ target/debug/cryptotax transactions.csv --currency ETH > eth.csv

Optionally, set the environment variable`RUST_LOG` to `info` or `debug` to see more logs.

    $ RUST_LOG=debug cargo run -- transactions_history.csv --currency ETH > eth.csv
