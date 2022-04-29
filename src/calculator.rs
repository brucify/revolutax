use std::io;
use std::ops::{Neg, Sub};
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::transaction::{Currency, Transaction, TransactionType};


// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug)]
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
    base: Currency,
    currency: Currency,
    costs: Vec<Cost>,
}

impl CostBook {
    fn new(currency: Currency, base: Currency) -> CostBook {
        let cash = Cash::new(base.clone(), Default::default());
        let cash_cost = Cost::new(Default::default(), cash);
        CostBook {
            base,
            currency,
            costs: vec![cash_cost],
        }
    }

    fn add_buy(&mut self, t: &Transaction) {
        if t.exchanged_currency.eq(&self.base) {
            self.find_cash_mut().map(|cost| {
                if let Money::Cash(cash) = &mut cost.exchanged {
                    cash.amount += t.exchanged_amount;
                    cost.paid_amount += t.paid_amount;
                }
            });
        } else {
            let coupon = Coupon::new(t.exchanged_currency.clone(), t.exchanged_amount, t.date.clone());
            let coupon_cost = Cost::new(t.paid_amount, coupon);
            self.costs.push(coupon_cost);
        }
    }

    // TODO
    fn add_sell(&mut self, transaction: &Transaction) {
        debug!("current costs: {:?}", self.costs);
        let income = transaction.to_money(&self.base);
        debug!("income for {:?} {:?}: {:?}", transaction.paid_amount, transaction.paid_currency, income);
        let costs = self.find_cost(&income, transaction.paid_amount).unwrap();
        debug!("costs: {:?}", costs);
        // TODO deduct found cost from self

        if transaction.exchanged_currency.eq(&self.base) {
            self.find_cash_mut().map(|cost| {
                if cost.paid_amount >= transaction.paid_amount {
                    // TODO tax report
                    let x = cost.deduct(&transaction.paid_amount)
                        .map(|cost| {
                            TaxableTransaction{
                                currency: transaction.paid_currency.clone(),
                                amount: transaction.paid_amount,
                                // income: transaction.to_money(&self.base),
                                income: Cash::new(transaction.exchanged_currency.clone(), transaction.exchanged_amount),
                                cost: vec![cost],
                                net_income: None
                            }
                        }).unwrap();
                    debug!("{:?}", x);
                } else {
                    // TODO partial tax report,
                    let x = cost.deduct(&cost.paid_amount.clone())
                        .map(|cost| {
                            TaxableTransaction{
                                currency: transaction.paid_currency.clone(),
                                amount: transaction.paid_amount,
                                // income: transaction.to_money(&self.base),
                                income: Cash::new(transaction.exchanged_currency.clone(), transaction.exchanged_amount),
                                cost: vec![cost],
                                net_income: None
                            }
                        }).unwrap();
                    debug!("{:?}", x);
                }
            });
        } else {
            let x = Coupon::new(transaction.exchanged_currency.clone(), transaction.exchanged_amount, transaction.date.clone());
        }
    }

    fn find_cash_mut(&mut self) -> Option<&mut Cost> {
        self.costs.iter_mut().find(|c| {
            match c.exchanged {
                Money::Cash(_) => true,
                Money::Coupon(_) => false
            }
        })
    }

    fn find_cost(&self, income: &Money, paid_amount: Decimal) -> io::Result<Vec<Cost>> {
        match income {
            Money::Cash(_) => do_find_costs(&self.costs, paid_amount, vec![maybe_cash_cost, maybe_coupon_cost]),
            Money::Coupon(_) => do_find_costs(&self.costs, paid_amount, vec![maybe_coupon_cost, maybe_cash_cost])
        }
    }
}

fn do_find_costs(costs: &Vec<Cost>, remaining: Decimal, funs: Vec<fn(&Cost, Decimal) -> Option<Cost>>) -> io::Result<Vec<Cost>> {
    let (remaining, costs) =
        funs.iter().fold((remaining, vec![]), |(remaining, acc), fun| {
            costs.iter().fold((remaining, acc), |(remaining, mut acc), cost| {
                if remaining.eq(&dec!(0)) { (remaining, acc) }
                else {
                    let amount = remaining.max(cost.paid_amount.neg());
                    match fun(cost, amount) {
                        None => (remaining, acc),
                        Some(cost) => {
                            acc.push(cost);
                            (remaining.sub(amount), acc)
                        }
                    }
                }
            })
        });
    remaining.eq(&dec!(0)).then(|| ()).ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
    Ok(costs)
}

fn maybe_coupon_cost(cost: &Cost, amount: Decimal) -> Option<Cost> {
    match &cost.exchanged {
        Money::Cash(_) => None,
        Money::Coupon(coupon) => {
            let coupon = Coupon::new(coupon.currency.clone(), coupon.amount / cost.paid_amount * amount.abs(), coupon.date.clone());
            let coupon_cost = Cost::new(amount.neg(), coupon);
            Some(coupon_cost)
        }
    }
}

fn maybe_cash_cost(cost: &Cost, amount: Decimal) -> Option<Cost> {
    match &cost.exchanged {
        Money::Coupon(_) => None,
        Money::Cash(cash) => {
            let cash = Cash::new(cash.currency.clone(), cash.amount / cost.paid_amount * amount.abs());
            let cash_cost = Cost::new(amount.neg(), cash);
            Some(cash_cost)
        }
    }
}

impl Transaction {
    fn to_money(&self, base: &Currency) -> Money {
        if self.exchanged_currency.eq(base) {
            Cash::new(self.exchanged_currency.clone(), self.exchanged_amount)
        } else {
            Coupon::new(self.exchanged_currency.clone(), self.exchanged_amount, self.date.clone())
        }
    }
}

#[derive(Debug)]
struct Cost {
    paid_amount: Decimal,
    exchanged: Money,
}

impl Cost {
    fn new(paid_amount: Decimal, exchanged: Money) -> Cost {
        Cost{ paid_amount, exchanged }
    }

    fn deduct(&mut self, paid_amount: &Decimal) -> Option<Money> {
        if let Money::Cash(c) = &mut self.exchanged {
            let deducted = Cash::new(c.currency.clone(), c.amount * paid_amount / self.paid_amount);
            c.amount += c.amount * paid_amount / self.paid_amount;
            self.paid_amount += paid_amount;
            Some(deducted)
        } else { None }
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
    let (txns, b) =
        txns.iter().fold((vec![], CostBook::new(currency.clone(), base.clone())),
                         |(acc, mut book), t| {
            match t.r#type {
                TransactionType::Buy => book.add_buy(t),
                TransactionType::Sell => book.add_sell(t),
            }
            (acc, book)
        });
    debug!("Current costs for {:?}:", b.currency);
    b.costs.iter().for_each(|c| debug!("{:?}", c));
    Ok(txns)
}
