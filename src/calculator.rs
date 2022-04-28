use std::io;
use chrono::Weekday::Mon;
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::transaction::{Currency, Transaction, TransactionType};


// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
pub(crate) struct TaxableTransaction {
    currency: Currency,             // Valutakod
    amount: Decimal,                // Antal
    income: Money,                  // Försäljningspris
    cost: Vec<Money>,               // Omkostnadsbelopp
    net_income: Option<Decimal>,    // Vinst/förlust
}

impl TaxableTransaction {
    fn new(currency: Currency, sales: Money) -> TaxableTransaction {
        TaxableTransaction{
            currency,
            income: sales,
            amount: Default::default(),
            cost: Default::default(),
            net_income: Default::default(),
        }
    }
}

#[derive(Debug)]
struct CostBook {
    currency: Currency,
    cost_in_cash: Cost,
    cost_in_coupons: Vec<Cost>,
}

impl CostBook {
    fn new(currency: Currency, base: Currency) -> CostBook {
        let cost_in_cash = Cost{
            paid_amount: Default::default(),
            exchanged: Cash::new(base, Default::default())
        };
        CostBook {
            currency,
            cost_in_cash: cost_in_cash,
            cost_in_coupons: vec![],
        }
    }

    fn add_buy(&mut self, t: &Transaction, base: &Currency) {
        if t.exchanged_currency.eq(base) {
            if let Money::Cash(cash) = &mut self.cost_in_cash.exchanged {
                cash.amount += t.exchanged_amount;
                self.cost_in_cash.paid_amount += t.paid_amount;
            }
        } else {
            self.cost_in_coupons.push(Cost{
                paid_amount: t.paid_amount,
                exchanged: Coupon::new(t.exchanged_currency.clone(), t.exchanged_amount, t.date.clone())
            });
        }
    }

    fn add_sell(&mut self, t: &Transaction, base: &Currency) {
        if t.exchanged_currency.eq(base) {
            if self.cost_in_cash.paid_amount >= t.paid_amount {
                self.cost_in_cash.deduct(&t.paid_amount);
            }
        }
    }

}

#[derive(Debug)]
struct Cost {
    paid_amount: Decimal,
    exchanged: Money,
}

impl Cost {
    fn deduct(&mut self, paid_amount: &Decimal) {
        if let Money::Cash(c) = &mut self.exchanged {
            c.amount += c.amount * paid_amount / self.paid_amount;
            self.paid_amount += paid_amount;
        }
    }
}

#[derive(Debug)]
enum Money {
    Cash(Cash),
    Coupon(Coupon),
}

#[derive(Debug)]
struct Cash { currency: Currency, amount: Decimal }

#[derive(Debug)]
struct Coupon { currency: Currency, amount: Decimal, date: String }

impl Cash {
    fn new(currency: Currency, amount: Decimal) -> Money {
        let cash = Cash{ currency, amount };
        Money::Cash(cash)
    }
}

impl Coupon {
    fn new(currency: Currency, amount: Decimal, date: String) -> Money {
        let coupon = Coupon{ currency, amount, date };
        Money::Coupon(coupon)
    }
}

pub(crate) async fn tax(txns: &Vec<Transaction>, currency: &Currency, base: &Currency) -> io::Result<Vec<TaxableTransaction>> {
    let book = CostBook::new(currency.clone(), base.clone());
    let (txns, book) =
        txns.iter().fold((vec![], book), |(acc, mut b), t| {
            match t.r#type {
                TransactionType::Buy => b.add_buy(t, base),
                TransactionType::Sell => b.add_sell(t, base),
            }
            (acc, b)
        });
    debug!("{:?}", book);
    Ok(txns)
}
