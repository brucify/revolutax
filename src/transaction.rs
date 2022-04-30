use rust_decimal::Decimal;

#[derive(Debug)]
pub(crate) struct Transaction {
    pub(crate) r#type: TransactionType,
    pub(crate) paid_currency: Currency,
    pub(crate) paid_amount: Decimal,
    pub(crate) exchanged_currency: Currency,
    pub(crate) exchanged_amount: Decimal,
    pub(crate) date: String,
}

impl Transaction {
    pub(crate) fn new() -> Transaction {
        Transaction{
            r#type: TransactionType::Buy,
            paid_currency: "".to_string(),
            paid_amount: Default::default(),
            exchanged_currency: "".to_string(),
            exchanged_amount: Default::default(),
            date: "".to_string()
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

#[derive(Debug)]
pub(crate) enum TransactionType {
    Buy,
    Sell
}

pub(crate) type Currency = String;


#[derive(Debug)]
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
}

#[derive(Debug)]
pub(crate) struct Cash {
    pub(crate) currency: Currency,
    pub(crate) amount: Decimal
}

#[derive(Debug)]
pub(crate) struct Coupon {
    pub(crate) currency: Currency,
    pub(crate) amount: Decimal,
    pub(crate) date: String
}