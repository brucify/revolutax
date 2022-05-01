use std::fmt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, PartialEq)]
pub(crate) struct Transaction {
    pub(crate) r#type: TransactionType,
    pub(crate) paid_currency: Currency,
    pub(crate) paid_amount: Decimal,
    pub(crate) exchanged_currency: Currency,
    pub(crate) exchanged_amount: Decimal,
    pub(crate) date: String,
    pub(crate) is_vault: bool,
}

impl Transaction {
    pub(crate) fn new() -> Transaction {
        Transaction{
            r#type: TransactionType::Buy,
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

#[derive(Debug, PartialEq)]
pub(crate) enum TransactionType {
    Buy,
    Sell
}
pub(crate) type Currency = String;

#[derive(Debug, PartialEq)]
pub(crate) enum Money {
    Cash(Cash),
    Coupon(Coupon),
}

impl Money {
    pub(crate)fn new_cash(currency: Currency, amount: Decimal) -> Money {
        let cash = Cash{ currency, amount };
        Money::Cash(cash)
    }

    pub(crate) fn new_coupon(currency: Currency, amount: Decimal, date: String) -> Money {
        let coupon = Coupon{ currency, amount, date };
        Money::Coupon(coupon)
    }

    pub(crate) fn is_cash(&self) -> bool {
        match self { Money::Cash(_) => true, Money::Coupon(_) => false }
    }

    pub(crate) fn amount(&self) -> Decimal {
        match self {
            Money::Cash(cash) => cash.amount,
            Money::Coupon(coupon) => coupon.amount
        }
    }

    pub(crate) fn deduct(&mut self, amount: Decimal) -> Money {
        match self {
            Money::Cash(cash) => {
                cash.amount -= amount;
                Money::new_cash(cash.currency.clone(), amount)
            },
            Money::Coupon(coupon) => {
                coupon.amount -= amount;
                Money::new_coupon(coupon.currency.clone(), amount, coupon.date.clone())
            }
        }
    }

    pub(crate) fn to_net_income(&self, costs: &Vec<Money>) -> Option<Decimal> {
        let all_cash = costs.iter().all(|c| c.is_cash());
        match (self, all_cash) {
            (Money::Cash(cash), true) => {
                let cost = costs.iter().fold(dec!(0), |acc, c| acc + c.amount());
                Some(cash.amount + cost)
            }
            _ => None
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Money::Cash(cash) => write!(f, "{}", cash.amount),
            Money::Coupon(coupon) => write!(f, "({} {} {})", coupon.amount, coupon.currency, coupon.date)
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Cash {
    pub(crate) currency: Currency,
    pub(crate) amount: Decimal
}

#[derive(Debug, PartialEq)]
pub(crate) struct Coupon {
    pub(crate) currency: Currency,
    pub(crate) amount: Decimal,
    pub(crate) date: String
}