/*
 * https://www.skatteverket.se/download/18.6e8a1495181dad540843eb2/1665748259651/SKV269_28_(2022P4).pdf
 */

use anyhow::{anyhow, Result};
use chrono::Datelike;
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

use crate::calculator::{Currency, TaxableTrade};

#[derive(Debug, Serialize)]
pub(crate) struct SruFile {
    // #BLANKETT
    // <BlankettTyp> Anger vilket blankettblock
    // som avses. Vid inlämning får värden enligt kolumnen ”Blankettblock” i
    // tabell 1, se ref[1]. Endast versaler, ”-” och siffror är tillåtna.
    form: String,

    // #IDENTITET
    identity: Identity,

    // #NAMN
    // <Namn> Namnet på dig so lämnar uppgifterna.Om
    // uppgiftelämnas visas den sedan på mottagningskvittensen. Längden på fältet får vara högst 250 tecken långt, dock används endast position 1 - 25 på
    // mottagningskvittensen.
    name: Option<String>,

    // #UPPGIFT
    information: Vec<Information>,

    // #SYSTEMINFO
    // Används till uppgiftslämnarens egna uppgifter. Endast en
    // post får lämnas. Skatteverket läser inte in posten.
    system_info: Option<String>,

    // #BLANKETTSLUT Markerar att blankettblocket slutar.
    // #FIL_SLUT Markerar att filen slutar.
}

#[derive(Debug, Serialize)]
struct Identity {
    // <OrgNr> Person-/organisations-/samordningsnummer för
    // den som uppgifterna avser. Anges i formen
    // SSÅÅMMDDNNNK.
    org_num: String,

    // <DatFramst> Datum för framställande av uppgifterna.
    // Anges i formen SSÅÅMMDD.

    // <TidFramst> Klockslag för framställande av uppgifterna.
    // Anges i formen TTMMSS.
}

#[derive(Debug, Serialize)]
struct Information {
    // <FältKod> Den fältkod som finns angiven i fältnamns-
    // tabellen för respektive blankettblock. Med några få
    // undantag är det fältkoder som finns på respektive
    // blankett. Förutom eventuell regel som anges i
    // fältnamnstabellen gäller att en fältkod får förekomma
    // endast en gång per blankettblock. #UPPGIFT får inte vara
    // blank utan ska innehålla en fältkod och värde.
    field_code: String,

    // <FältVärde> Det värde som ska redovisas för fältkoden.
    field_value: String,
}

impl SruFile {
    pub(crate) fn try_new(
        trades: Vec<&TaxableTrade>,
        org_num: String,
        name: Option<String>,
    ) -> Option<Self> {
        trades_to_sru_information(trades)
            .map(|information| {
                SruFile {
                    form: format!("K4-{}P4", chrono::Utc::now().year() - 1),
                    identity: Identity { org_num },
                    name,
                    information,
                    system_info: None,
                }
            })
    }

    pub(crate) fn write(&self, mut handle: impl Write) -> Result<()> {
        writeln!(handle, "#BLANKETT {}", self.form)?;

        let now = chrono::Utc::now();

        let identity = &self.identity;
        writeln!(
            handle,
            "#IDENTITET {} {} {}",
            identity.org_num, now.format("%Y%m%d").to_string(), now.format("%H%M%S").to_string()
        )?;

        if let Some(name) = &self.name {
            writeln!(handle, "#NAMN {}", name)?;
        }

        for info in &self.information {
            writeln!(handle, "#UPPGIFT {} {}", info.field_code, info.field_value)?;
        }

        writeln!(handle, "#BLANKETTSLUT")?;
        writeln!(handle, "#FIL_SLUT")?;

        Ok(())
    }
}

fn trades_to_sru_information(
    trades: Vec<&TaxableTrade>,
) -> Option<Vec<Information>> {
    let mut summary_map: HashMap<Currency, (Decimal, Decimal, Decimal)> = HashMap::new();

    let mut err = Err(anyhow!("All costs must be cash"));
    for trade in trades {
        if trade.costs.iter().all(|c| c.is_cash()) {
            let (amount, income, costs) = summary_map.entry(trade.currency.clone())
                .or_insert((dec!(0), dec!(0), dec!(0)));
            *amount += trade.amount;
            *income += trade.income.amount();
            *costs += trade.costs.iter().map(|x| x.amount()).sum::<Decimal>();
            err = Ok(());
        }
    }
    err.ok()?;

    let mut result = vec![];

    for (i, (currency, (amount, income, costs))) in summary_map.into_iter().enumerate() {
        let net_income = income + costs;
        debug!("i: {}", i);
        debug!("Currency: {}", currency);
        debug!("Total Amount: {}", amount);
        debug!("Total Income: {}", income);
        debug!("Total Costs: {}", costs);
        debug!("Net Income: {}", income + costs);
        debug!("-----------------------");
        let info_vec = sru_information_vec(i+1, currency, amount, income, costs, net_income);
        result.extend(info_vec);
    }

    Some(result)
}

fn sru_information_vec(
    i: usize,
    currency: Currency,
    amount: Decimal,
    income: Decimal,
    costs: Decimal,
    net_income: Decimal
) -> Vec<Information> {
    let mut info_vec = vec![];
    info_vec.push(Information { field_code: format!("34{}0", i), field_value: amount.abs().round().to_string() });                  // D.1 Antal/Belopp i utländsk valuta
    info_vec.push(Information { field_code: format!("34{}1", i), field_value: currency.to_string() });                              // D.1 Beteckning/Valutakod
    info_vec.push(Information { field_code: format!("34{}2", i), field_value: income.abs().round().to_string() });                  // D.1 Försäljningspris/Återbetalat belopp omräknat till svenska kronor
    info_vec.push(Information { field_code: format!("34{}3", i), field_value: costs.abs().round().to_string() });                   // D.1 Omkostnadsbelopp/Utlånat belopp omräknat till svenska kronor

    match net_income.is_sign_positive() {
        true => info_vec.push(Information { field_code: format!("34{}4", i), field_value: net_income.abs().round().to_string() }),  // D.1 Vinst
        false => info_vec.push(Information { field_code: format!("34{}5", i), field_value: net_income.abs().round().to_string() }), // D.1 Förlust
    }

    info_vec
}

#[cfg(test)]
mod test {
    use crate::calculator::TaxableTrade;
    use crate::reader::RevolutRow2023;
    use crate::skatteverket::sru_file::{Identity, SruFile, trades_to_sru_information};
    use futures::executor::block_on;
    use std::io::Write;
    use std::path::PathBuf;
    use anyhow::anyhow;

    #[test]
    fn should_write_sru_file() -> anyhow::Result<()> {
        /*
         * Given
         */
        let mut file = tempfile::NamedTempFile::new()?;
        // Current: Buy 30, Sell 30 (Trade 1), Buy 50, Transfer out 10, Transfer in 100, Sell 50 (Trade 2), Spend 25 (Trade 3) - Balance 65
        // Savings: Transfer in 10, Buy 20, Buy 40, Buy 60, Buy 80, Transfer out 100 - Balance 110
        writeln!(file, "
            Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance
            EXCHANGE,Current,2023-01-01 10:00:00,2023-01-01 10:00:00,Exchanged to EOS,30.0000,EOS,600.00,609.15,9.15,SEK,COMPLETED,30.0000
            EXCHANGE,Current,2023-01-02 10:00:00,2023-01-02 10:00:00,Exchanged to SEK,-30.0000,EOS,-400.00,-394.86,5.14,SEK,COMPLETED,0.0000
            EXCHANGE,Current,2023-02-01 12:00:00,2023-02-01 12:00:00,Exchanged to EOS,50.0000,EOS,1000.00,1009.65,9.65,SEK,COMPLETED,50.0000
            TRANSFER,Current,2023-02-08 10:00:00,2023-02-08 10:00:00,Transferred to Savings,-10.0000,EOS,-200.00,-200.00,0.00,SEK,COMPLETED,40.0000
            TRANSFER,Current,2023-04-04 10:00:00,2023-04-04 10:00:00,Transferred to Current,100.0000,EOS,2000.00,2000.00,0.00,SEK,COMPLETED,140.0000
            EXCHANGE,Current,2023-04-04 11:00:00,2023-04-04 11:00:00,Exchanged to SEK,-50.0000,EOS,-600.00,-594.86,5.14,SEK,COMPLETED,90.0000
            CARD_PAYMENT,Current,2023-05-06 10:00:00,2023-05-06 10:00:00,Payment to Amazon,-25.0000,EOS,-500.00,-495.75,4.25,SEK,COMPLETED,65.0000
            TRANSFER,Savings,2023-02-08 10:00:00,2023-02-08 10:00:00,Transferred from Current,10.0000,EOS,200.00,200.00,0.00,SEK,COMPLETED,10.0000
            EXCHANGE,Savings,2023-03-01 14:00:00,2023-03-01 14:00:00,Exchanged to EOS,20.0000,EOS,400.00,404.57,4.57,SEK,COMPLETED,30.0000
            EXCHANGE,Savings,2023-03-02 14:00:00,2023-03-02 14:00:00,Exchanged to EOS,40.0000,EOS,800.00,809.15,9.15,SEK,COMPLETED,70.0000
            EXCHANGE,Savings,2023-03-03 14:00:00,2023-03-03 14:00:00,Exchanged to EOS,60.0000,EOS,1200.00,1213.73,13.73,SEK,COMPLETED,130.0000
            EXCHANGE,Savings,2023-03-04 14:00:00,2023-03-04 14:00:00,Exchanged to EOS,80.0000,EOS,1600.00,1618.31,18.31,SEK,COMPLETED,210.0000
            TRANSFER,Savings,2023-04-04 10:00:00,2023-04-04 10:00:00,Transferred to Current,-100.0000,EOS,-2000.00,-2000.00,0.00,SEK,COMPLETED,110.0000
        ")?;
        let path = file.path().to_str().unwrap();

        /*
         * When
         */
        let taxable_trades = block_on(async {
            let trades = RevolutRow2023::deserialize_from(&PathBuf::from(path)).await?;
            TaxableTrade::taxable_trades(&trades, &"EOS".to_string(), &"SEK".to_string()).await
        })?;

        let sru_file = SruFile {
            form: "K4-2022P4".to_string(),
            identity: Identity { org_num: "195001011234".to_string() },
            name: None,
            information: trades_to_sru_information(taxable_trades.iter().collect()).ok_or(anyhow!(""))?,
            system_info: None,
        };

        let mut buf = vec![];
        sru_file.write(&mut buf)?;

        /*
         * Then
         */
        let output = String::from_utf8(buf).unwrap();

        // let stdout = std::io::stdout();
        // let handle = stdout.lock();
        // sru_file.write(handle)?;

        assert!(output.starts_with("#BLANKETT K4-2022P4\n"));
        assert!(output.contains("#IDENTITET 195001011234 "));
        assert!(output.contains("#UPPGIFT 3410 105\n"));
        assert!(output.contains("#UPPGIFT 3411 EOS\n"));
        assert!(output.contains("#UPPGIFT 3412 1485\n"));
        assert!(output.contains("#UPPGIFT 3413 2125\n"));
        assert!(output.contains("#UPPGIFT 3415 639\n"));
        assert!(output.ends_with("#BLANKETTSLUT\n#FIL_SLUT\n"));

        Ok(())
    }
}