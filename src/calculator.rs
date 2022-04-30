use std::io;
use std::ops::{Neg, Sub};
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::transaction::{Currency, Transaction, TransactionType, Money};


// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug)]
pub(crate) struct TaxableTransaction {
    date: String,
    currency: Currency,             // Valutakod
    amount: Decimal,                // Antal
    income: Money,                  // Försäljningspris
    costs: Vec<Money>,               // Omkostnadsbelopp
    net_income: Option<Decimal>,    // Vinst/förlust
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
}

#[derive(Debug)]
struct CostBook {
    base: Currency,
    currency: Currency,
    costs: Vec<Cost>,
}

impl CostBook {
    fn new(currency: Currency, base: Currency) -> CostBook {
        CostBook {
            base,
            currency,
            costs: vec![],
        }
    }

    fn add_buy(&mut self, t: &Transaction) {
        match t.to_money(&self.base) {
            Money::Cash(income) => {
                self.find_cash_cost_mut().map(|cost| {
                    if let Money::Cash(cash) = &mut cost.exchanged {
                        cash.amount += income.amount;
                        cost.paid_amount += t.paid_amount;
                    }
                });
            }
            income@ Money::Coupon(_) => {
                let coupon_cost = Cost::new(t.paid_amount, income);
                self.costs.push(coupon_cost);
            }
        }
    }

    fn add_sell(&mut self, transaction: &Transaction) -> io::Result<TaxableTransaction> {
        let income = transaction.to_money(&self.base);
        let costs = self.find_and_deduct_cost(&income, transaction.paid_amount)?;
        let costs = costs.into_iter().map(|c| c.exchanged).collect();
        Ok(TaxableTransaction{
            date: transaction.date.clone(),
            currency: transaction.paid_currency.clone(),
            amount: transaction.paid_amount,
            income,
            costs,
            net_income: None // TODO
        })
    }

    fn find_cash_cost_mut(&mut self) -> Option<&mut Cost> {
        match self.costs.iter().find(|c| c.exchanged.is_cash()) {
            None => {
                let cash = Money::new_cash(self.base.clone(), Default::default());
                let cash_cost = Cost::new(Default::default(), cash);
                self.costs.push(cash_cost);
                self.costs.last_mut()
            }
            Some(_) => {
                self.costs.iter_mut().find(|c| c.exchanged.is_cash())
            }
        }
    }

    fn find_and_deduct_cost(&mut self, income: &Money, paid_amount: Decimal) -> io::Result<Vec<Cost>> {
        match income {
            Money::Cash(_) => do_find_and_deduct_cost(&mut self.costs, paid_amount, vec![deduct_cash_cost, deduct_coupon_cost]),
            Money::Coupon(_) => do_find_and_deduct_cost(&mut self.costs, paid_amount, vec![deduct_coupon_cost, deduct_cash_cost])
        }
    }
}

fn do_find_and_deduct_cost(costs: &mut Vec<Cost>, remaining: Decimal, funs: Vec<fn(&mut Cost, Decimal) -> Option<Cost>>) -> io::Result<Vec<Cost>> {
    let (remaining, costs): (Decimal, Vec<Cost>) =
        funs.iter().fold((remaining, vec![]), |(remaining, acc), fun| {
            let (remaining, acc) = costs.iter_mut().fold((remaining, acc), |(remaining, mut acc), cost| {
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
            });
            costs.retain(|c| !c.paid_amount.is_zero());
            (remaining, acc)
        });
    remaining.eq(&dec!(0)).then(|| ()).ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
    Ok(costs)
}

fn deduct_coupon_cost(cost: &mut Cost, amount: Decimal) -> Option<Cost> {
    match &mut cost.exchanged {
        Money::Cash(_) => None,
        Money::Coupon(coupon) => {
            let used_cost_amount = coupon.amount / cost.paid_amount * amount.abs();
            let paid_amount = amount.neg();
            let c = Money::new_coupon(coupon.currency.clone(), used_cost_amount, coupon.date.clone());
            let coupon_cost = Cost::new(paid_amount, c);

            // deduct used cost from current cost
            coupon.amount -= used_cost_amount;
            cost.paid_amount -= paid_amount;

            // return deducted cost
            Some(coupon_cost)
        }
    }
}

fn deduct_cash_cost(cost: &mut Cost, amount: Decimal) -> Option<Cost> {
    match &mut cost.exchanged {
        Money::Coupon(_) => None,
        Money::Cash(cash) => {
            let used_cost_amount = cash.amount / cost.paid_amount * amount.abs();
            let paid_amount = amount.neg();
            let c = Money::new_cash(cash.currency.clone(), used_cost_amount);
            let cash_cost = Cost::new(paid_amount, c);

            // deduct used cost from current cost
            cash.amount -= used_cost_amount;
            cost.paid_amount -= paid_amount;

            // return deducted cost
            Some(cash_cost)
        }
    }
}

pub(crate) async fn tax(txns: &Vec<Transaction>, currency: &Currency, base: &Currency) -> io::Result<Vec<TaxableTransaction>> {
    let book = CostBook::new(currency.clone(), base.clone());
    let (txns, b) =
        txns.iter().fold((vec![], book), |(mut acc, mut book), t| {
            match t.r#type {
                TransactionType::Buy => book.add_buy(t),
                TransactionType::Sell => {
                    let x = book.add_sell(t).unwrap();
                    acc.push(x);
                },
            }
            (acc, book)
        });
    debug!("Remaining costs for {:?}:", b.currency);
    b.costs.iter().for_each(|c| debug!("{:?}", c));
    debug!("Taxable transactions:");
    txns.iter().for_each(|t| debug!("{:?}", t));
    Ok(txns)
}
