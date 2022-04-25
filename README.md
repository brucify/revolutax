cryptotax
=====

A Rust CLI application

```shell
cryptotax
Search for currency exchanges in a Revolut csv file and output a new csv containing the tax
information

USAGE:
    cryptotax [OPTIONS] <PATH>

ARGS:
    <PATH>    Path to the csv file that contains transactions.

OPTIONS:
    -c, --currency <CURRENCY>    Give 'ALL' for all currencies.
    -e, --exchanges              Print to stdout a new csv file with transactions with type
                                 'Exchange'
    -h, --help                   Print help information
```


Build
-----

    $ cargo build

Run
-----

    $ cargo run -- transactions_history.csv --currency ETH > eth.csv

Or

    $ cargo build
    $ target/debug/cryptotax transactions.csv --currency ETH > eth.csv

Optionally, set the environment variable`RUST_LOG` to `info` or `debug` to see more logs.

    $ RUST_LOG=debug cargo run -- transactions_history.csv --currency ETH > eth.csv
