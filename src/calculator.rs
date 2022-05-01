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

#[derive(Debug, PartialEq)]
struct Cost {
    paid_amount: Decimal,
    exchanged: Money,
    is_vault: bool,
}

impl Cost {
    fn new(paid_amount: Decimal, exchanged: Money, is_vault: bool) -> Cost {
        Cost{ paid_amount, exchanged, is_vault }
    }

    fn deduct(&mut self, paid_amount: Decimal) -> Option<Cost> {
        let exchanged_amount = self.exchanged.amount() / self.paid_amount * paid_amount.abs();
        let deducted = self.exchanged.deduct(exchanged_amount);
        self.paid_amount += paid_amount;
        let deducted_cost = Cost::new(paid_amount.neg(), deducted, self.is_vault);
        Some(deducted_cost)
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

    fn add_buy(&mut self, transaction: &Transaction) {
        match transaction.to_money(&self.base) {
            Money::Cash(income) => {
                self.find_cash_cost_mut(transaction.is_vault).map(|cost| {
                    if let Money::Cash(cash) = &mut cost.exchanged {
                        cash.amount += income.amount;
                        cost.paid_amount += transaction.paid_amount;
                    }
                });
            }
            income @ Money::Coupon(_) => {
                let coupon_cost = Cost::new(transaction.paid_amount, income, transaction.is_vault);
                self.costs.push(coupon_cost);
            }
        }
    }

    fn add_sell(&mut self, transaction: &Transaction) -> io::Result<TaxableTransaction> {
        let income = transaction.to_money(&self.base);
        let costs =
            self.find_and_deduct_cost(&income, transaction.paid_amount)?
                .into_iter()
                .map(|c| c.exchanged)
                .collect();
        Ok(TaxableTransaction{
            date: transaction.date.clone(),
            currency: transaction.paid_currency.clone(),
            amount: transaction.paid_amount,
            income,
            costs,
            net_income: None // TODO
        })
    }

    fn find_cash_cost_mut(&mut self, is_vault: bool) -> Option<&mut Cost> {
        match self.costs.iter().find(|c| c.exchanged.is_cash() && c.is_vault == is_vault) {
            None => {
                let cash = Money::new_cash(self.base.clone(), Default::default());
                let cash_cost = Cost::new(Default::default(), cash, is_vault);
                self.costs.push(cash_cost);
                self.costs.last_mut()
            }
            Some(_) => {
                self.costs.iter_mut().find(|c| c.exchanged.is_cash() && c.is_vault == is_vault)
            }
        }
    }

    fn find_and_deduct_cost(&mut self, income: &Money, paid_amount: Decimal) -> io::Result<Vec<Cost>> {
        match income {
            Money::Cash(_) => self.do_find_and_deduct_cost(
                paid_amount,
                vec![deduct_cash_cost, deduct_coupon_cost, deduct_vault_cash_cost, deduct_vault_coupon_cost]
            ),
            Money::Coupon(_) => self.do_find_and_deduct_cost(
                paid_amount,
                vec![deduct_coupon_cost, deduct_cash_cost, deduct_vault_coupon_cost, deduct_vault_cash_cost]
            )
        }
    }

    fn do_find_and_deduct_cost(&mut self, remaining: Decimal, funs: Vec<fn(&mut Cost, Decimal) -> Option<Cost>>) -> io::Result<Vec<Cost>> {
        let (remaining, costs): (Decimal, Vec<Cost>) =
            funs.iter().fold((remaining, vec![]), |(remaining, acc), fun| {
                let (remaining, acc) = self.costs.iter_mut().rev().fold((remaining, acc), |(remaining, mut acc), cost| {
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
                self.costs.retain(|c| !c.paid_amount.is_zero());
                (remaining, acc)
            });
        remaining.eq(&dec!(0)).then(|| ()).ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
        Ok(costs)
    }
}


fn deduct_coupon_cost(cost: &mut Cost, paid_amount: Decimal) -> Option<Cost> {
    match (&cost.exchanged, cost.is_vault) {
        (Money::Coupon(_), false) => cost.deduct(paid_amount),
        _ => None,
    }
}

fn deduct_cash_cost(cost: &mut Cost, paid_amount: Decimal) -> Option<Cost> {
    match (&cost.exchanged, cost.is_vault) {
        (Money::Cash(_), false) => cost.deduct(paid_amount),
        _ => None,
    }
}

fn deduct_vault_coupon_cost(cost: &mut Cost, paid_amount: Decimal) -> Option<Cost> {
    match (&cost.exchanged, cost.is_vault) {
        (Money::Coupon(_), true) => cost.deduct(paid_amount),
        _ => None,
    }
}

fn deduct_vault_cash_cost(cost: &mut Cost, paid_amount: Decimal) -> Option<Cost> {
    match (&cost.exchanged, cost.is_vault) {
        (Money::Cash(_), true) => cost.deduct(paid_amount),
        _ => None,
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


#[cfg(test)]
mod test {
    use crate::calculator::{Cost, CostBook};
    use crate::transaction::{Money, Transaction, TransactionType};
    use rust_decimal_macros::dec;
    use std::error::Error;

    #[test]
    fn should_add_buy() -> Result<(), Box<dyn Error>> {
        let mut book = CostBook::new("DOGE".to_string(), "SEK".to_string());

        let txn = Transaction{
            r#type: TransactionType::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(39.94),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-20),
            date: "2021-11-11 18:03:13".to_string(),
            is_vault: true
        };
        book.add_buy(&txn);

        let txn = Transaction{
            r#type: TransactionType::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(2000),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-5080.60),
            date: "2021-12-31 17:54:48".to_string(),
            is_vault: false
        };
        book.add_buy(&txn);

        let txn = Transaction{
            r#type: TransactionType::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(200),
            exchanged_currency: "EOS".to_string(),
            exchanged_amount: dec!(-500),
            date: "2022-02-03 10:30:29".to_string(),
            is_vault: false
        };
        book.add_buy(&txn);

        let txn = Transaction{
            r#type: TransactionType::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(30.3),
            exchanged_currency: "EOS".to_string(),
            exchanged_amount: dec!(-62.35),
            date: "2022-02-04 11:01:35".to_string(),
            is_vault: false
        };
        book.add_buy(&txn);

        let mut iter = book.costs.iter();
        assert_eq!(iter.next(), Some(&Cost{
            paid_amount: dec!(39.94),
            exchanged: Money::new_cash("SEK".to_string(), dec!(-20)),
            is_vault: true
        }));
        assert_eq!(iter.next(), Some(&Cost{
            paid_amount: dec!(2000),
            exchanged: Money::new_cash("SEK".to_string(), dec!(-5080.6)),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(&Cost{
            paid_amount: dec!(200),
            exchanged: Money::new_coupon("EOS".to_string(), dec!(-500), "2022-02-03 10:30:29".to_string()),
            is_vault: false
        }));
        assert_eq!(iter.next(), Some(&Cost{
            paid_amount: dec!(30.3),
            exchanged: Money::new_coupon("EOS".to_string(), dec!(-62.35), "2022-02-04 11:01:35".to_string()),
            is_vault: false
        }));
        assert_eq!(iter.next(), None);

        Ok(())
    }
}
