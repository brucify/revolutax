# revolutax

A Rust CLI tool for calculating taxes on cryptocurrencies traded on Revolut. It reads the 
account statement CSV file from Revolut and generates a new CSV file summarizing the taxable
trades.

This tools can also generate an [SRU file](https://www.skatteverket.se/privat/deklaration/lamnaenbilagatilldeklarationen.4.515a6be615c637b9aa46366.html?q=sru+fil) 
that can be uploaded to Swedish Tax Agency
([K4-bilagan - Försäljning - Värdepapper m.m.](https://skatteverket.se/privat/skatter/vardepapper/deklareraaktierochovrigavardepapper/deklareravardepapperexempel.4.7afdf8a313d3421e9a9519.html))
Although this program is specifically designed for reporting Swedish taxes, it can also be used
for general tax reporting purposes in other countries.

## Installation

To install Rust, follow the official installation guide available at [rustup.rs](https://rustup.rs/).

After Rust is installed, clone the repository and build the project:


    $ cargo build

## Usage

The program reads the transactions of type `EXCHANGE` and `CARD_PAYMENT` and generates a new csv file tax.csv. For example:

    $ cargo run -- account_statement.csv > tax.csv

Here is an example input CSV file `account_statement.csv`:

```csv
Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
EXCHANGE,Current,2023-01-01 10:00:00,2023-01-01 10:00:00,Exchanged to EOS,100.0000,EOS,600.00,609.15,9.15,SEK,COMPLETED,100.0000
EXCHANGE,Savings,2023-01-01 11:00:00,2023-01-01 11:00:00,Exchanged to EOS,50.0000,EOS,300.00,304.15,4.15,SEK,COMPLETED,50.0000
EXCHANGE,Current,2023-01-02 10:00:00,2023-01-02 10:00:00,Exchanged to SEK,-30.0000,EOS,-400.00,-394.86,5.14,SEK,COMPLETED,70.0000
CARD_PAYMENT,Current,2023-05-06 10:00:00,2023-05-06 10:00:00,Payment to Amazon,-25.0000,EOS,-500.00,-495.75,4.25,SEK,COMPLETED,45.0000
```

The program generates a tax report `tax.csv` that looks like this:

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

### Swedish Tax Agency

To generate an SRU file for the Skatteverket, provide the `--sru-file` flag. This will
produce a file with a .sru extension, containing the same capital gain information as
the CSV output, and is equivalent to a filled
[K4-bilagan](https://skatteverket.se/privat/skatter/vardepapper/deklareraaktierochovrigavardepapper/deklareravardepapperexempel.4.7afdf8a313d3421e9a9519.html)
form. The generated SRU file is formatted according to the [Skatteverket's specifications](https://www.skatteverket.se/download/18.6e8a1495181dad540843eb2/1665748259651/SKV269_28_(2022P4).pdf)
and includes relevant headers such as `#BLANKETT` and `#UPPGIFT`.

The `--sru-org-num` flag should also be provided, followed by the personal/organization number of
the taxpayer. This number is included in the `#IDENTITET` header, which is required by 
the Skatteverket. For example:

    $ cargo run -- \
        --sru-file \
        --sru-org-num 195012310123 \
        --sru-org-name "Svea Specimen" \
        --year-traded 2023 \
        --sum \
        revolut-2023.csv > BLANKETTER.sru

With `--sru-file` flag, the program generates an SRU file `BLANKETTER.sru`:

```
#BLANKETT K4-2022P4
#IDENTITET 195012310123 20230428 222030
#NAMN Svea Specimen
#UPPGIFT 7014 1
#UPPGIFT 3410 55
#UPPGIFT 3411 EOS
#UPPGIFT 3412 891
#UPPGIFT 3413 335
#UPPGIFT 3414 556
#BLANKETTSLUT
#FIL_SLUT
```

The generated SRU file can be submitted to the Skatteverket electronically, simplifying
the tax reporting process for the taxpayer. If the `--sru-file` flag is not used, the program
will produce a CSV file containing the same information.

### Available Options

FLAGS:
* `--print-exchanges-only`   (2022 csv only)Filter the input CSV file to show only items of type 'Exchange', and print to stdout
* `--print-trades`           (2022 csv only) Merge two lines of a currency exchange into a single trade, and print to stdout
* `--sru-file`               Print taxable trades in the Swedish Tax Agency's SRU file format
* `--sum`                    Summarize taxable trades by currency ("[genomsnittsmetoden](https://skatteverket.se/privat/skatter/vardepapper/andratillgangar/kryptovalutor.4.15532c7b1442f256bae11b60.html?q=kryptovalutor)")
* `-h, --help`                   Print help

OPTIONS:
* `-c, --currency <CURRENCY>`                  (2022 csv only) Specify the traded cryptocurrency to report the tax for. Use 'ALL' to show all currencies when using --print-exchanges-only
* `-b, --base-currency <BASE_CURRENCY>`        (2022 csv only) Specify the base fiat currency to report the tax in. Defaults to 'SEK'
* `--sru-org-name <SRU_ORG_NAME>`          Name to print in the SRU file
* `--sru-org-num <SRU_ORG_NUM>`            Personal/organisational number to print in the SRU file
* `--csv-version <CSV_VERSION>`            Specify the year of the Revolut CSV file to process. Defaults to 2023
* `--year-traded <YEAR_TRADED>`            Only include taxable trades from this year

## License
```
This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
```
