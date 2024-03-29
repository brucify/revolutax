use rust_decimal::Decimal;
use serde::{Serialize};

use super::{Currency, Money};

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct Trade {
    #[serde(rename = "Type")]
    pub(crate) direction: Direction,

    // Exchange fiat "SEK" in order to get paid in crypto "BTC"
    #[serde(rename = "Paid Currency")]
    pub(crate) paid_currency: Currency,

    #[serde(rename = "Paid Amount")]
    pub(crate) paid_amount: Decimal,

    // Exchange fiat "SEK" in order to get paid in crypto "BTC"
    #[serde(rename = "Exchanged Currency")]
    pub(crate) exchanged_currency: Currency,

    #[serde(rename = "Exchanged Amount")]
    pub(crate) exchanged_amount: Decimal,

    #[serde(rename = "Date")]
    pub(crate) date: String,

    #[serde(rename = "Vault")]
    pub(crate) is_vault: bool,
}

impl Trade {
    pub(crate) fn new() -> Trade {
        Trade {
            direction: Direction::Buy,
            paid_currency: "".to_string(),
            paid_amount: Default::default(),
            exchanged_currency: "".to_string(),
            exchanged_amount: Default::default(),
            date: "".to_string(),
            is_vault: false
        }
    }

    pub(crate) fn to_money(&self, base: &Currency) -> Money {
        if self.exchanged_currency.eq(base) {
            Money::new_cash(self.exchanged_currency.clone(), self.exchanged_amount)
        } else {
            Money::new_coupon(self.exchanged_currency.clone(), self.exchanged_amount, self.date.clone())
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum Direction {
    Buy,
    Sell
}
