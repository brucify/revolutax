use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::calculator::Currency;

#[derive(Debug, PartialEq, Clone)]
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
                let sum = costs.iter().fold(dec!(0), |acc, c| acc + c.amount());
                Some(cash.amount + sum)
            }
            _ => None
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Money::Cash(cash) => write!(f, "{}", cash.amount),
            Money::Coupon(coupon) => write!(f, "({} {} {})", coupon.amount, coupon.currency, coupon.date)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Cash {
    currency: Currency,
    pub(crate) amount: Decimal
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Coupon {
    currency: Currency,
    amount: Decimal,
    date: String
}