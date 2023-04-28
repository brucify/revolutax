use crate::calculator::Currency;
use crate::calculator::money::Money;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug, PartialEq)]
pub(crate) struct TaxableTrade {
    date: String,
    pub(crate) currency: Currency,             // Valutakod
    pub(crate) amount: Decimal,                // Antal
    pub(crate) income: Money,                  // Försäljningspris
    pub(crate) costs: Vec<Money>,              // Omkostnadsbelopp
    net_income: Option<Decimal>,    // Vinst/förlust
}

impl Serialize for TaxableTrade {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer,
    {
        // 6 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("TaxableTrade", 6)?;
        state.serialize_field("Date", &self.date)?;
        state.serialize_field("Currency", &self.currency)?;
        state.serialize_field("Amount", &self.amount)?;
        state.serialize_field("Income", &format!("{}", self.income))?;
        state.serialize_field("Cost", &self.costs_to_string())?;
        state.serialize_field("Net Income", &self.net_income)?;
        state.end()
    }
}

impl TaxableTrade {
    pub(crate) fn new(
        date: String,
        currency: Currency,
        amount: Decimal,
        income: Money,
        costs: Vec<Money>,
        net_income: Option<Decimal>,
    ) -> TaxableTrade {
        TaxableTrade {
            date,
            currency,
            amount,
            income,
            costs,
            net_income,
        }
    }

    fn costs_to_string(&self) -> String {
        if self.costs.iter().all(|c| c.is_cash()) {
            self.costs.iter()
                .fold(dec!(0), |acc, c| acc + c.amount())
                .to_string()
        } else {
            self.costs.iter()
                .fold("".to_string(), |acc, c| format!("{}, {}", acc, c))
        }
    }
}

