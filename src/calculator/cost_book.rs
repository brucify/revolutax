use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::ops::{Neg, Sub};

use super::{Currency, TaxableTrade, Money, Trade};

#[derive(Debug)]
pub(crate) struct CostBook {
    pub(crate) base_currency: Currency,
    pub(crate) currency: Currency,
    pub(crate) costs: Vec<Cost>,
}

impl CostBook {
    pub(crate) fn new(currency: Currency, base_currency: Currency) -> CostBook {
        CostBook {
            base_currency,
            currency,
            costs: vec![],
        }
    }

    pub(crate) fn add_buy(&mut self, trade: &Trade) {
        match trade.to_money(&self.base_currency) {
            Money::Cash(cash) => {
                self.find_and_add_cash(
                    trade.is_vault,
                    trade.paid_amount,
                    cash.amount
                );
            }
            cost @ Money::Coupon(_) => {
                self.costs.push(
                    Cost::new(
                        trade.paid_amount,
                        cost,
                        trade.is_vault
                    )
                );
            }
        }
    }

    pub(crate) fn add_sell(&mut self, trade: &Trade) -> Result<TaxableTrade> {
        let income = trade.to_money(&self.base_currency);

        let costs =
            self.find_and_deduct_cost(&income, trade.paid_amount)?
                .into_iter()
                .map(|c| c.exchanged)
                .collect();

        let net_income = income.to_net_income(&costs);
        
        Ok(
            TaxableTrade::new(
                Some(trade.date.clone()),
                trade.paid_currency.clone(),
                trade.paid_amount,
                income,
                costs,
                net_income
            )
        )
    }

    fn find_and_add_cash(&mut self, is_vault: bool, paid_amount: Decimal, amount: Decimal) {
        if let Some(cash_cost) =
            self.costs.iter_mut()
                .find(|c| c.exchanged.is_cash() && c.is_vault == is_vault)
        {
            cash_cost.add_cash(paid_amount, amount);
        } else {
            self.costs.push(
                Cost::new(
                    paid_amount,
                    Money::new_cash(self.base_currency.clone(), amount),
                    is_vault
                )
            );
        }
    }

    /// Find the costs for the given `income`. Then deduct them from the book.
    /// If `income` is `Money::Cash`, try deduct from the cash in the `CostBook`.
    /// Likewise, if `income` is `Money::Coupon`, try deduct from the coupons in the `CostBook`.
    /// Only start deducting from the vault if there are no non-vault costs to deduct.
    /// Returns a `Vec<Cost>` which is a list of deducted costs.
    fn find_and_deduct_cost(&mut self, income: &Money, paid_amount: Decimal) -> Result<Vec<Cost>> {
        let mut deductor = Deductor::new(&mut self.costs, paid_amount);
        let deducted =
            match income {
                Money::Cash(_) =>
                    deductor.maybe_deduct(Cost::maybe_deduct_cash_cost)
                        .maybe_deduct(Cost::maybe_deduct_coupon_cost)
                        .maybe_deduct(Cost::maybe_deduct_vault_cash_cost)
                        .maybe_deduct(Cost::maybe_deduct_vault_coupon_cost)
                        .collect(),
                Money::Coupon(_) =>
                    deductor.maybe_deduct(Cost::maybe_deduct_coupon_cost)
                        .maybe_deduct(Cost::maybe_deduct_cash_cost)
                        .maybe_deduct(Cost::maybe_deduct_vault_coupon_cost)
                        .maybe_deduct(Cost::maybe_deduct_vault_cash_cost)
                        .collect(),
            };

        match deductor.remaining.eq(&dec!(0)) {
            true => Ok(deducted),
            false => Err(anyhow!("Not enough costs to deduct from"))
        }
    }

}


#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Cost {
    paid_amount: Decimal,
    exchanged: Money,
    is_vault: bool,
}

impl Cost {
    fn new(paid_amount: Decimal, exchanged: Money, is_vault: bool) -> Cost {
        Cost{ paid_amount, exchanged, is_vault }
    }

    fn maybe_deduct(&mut self, paid_amount: Decimal) -> Option<Cost> {
        if self.paid_amount + paid_amount < dec!(0) {
            None
        } else {
            let exchanged_amount = self.exchanged.amount() / self.paid_amount * paid_amount.abs();
            let deducted = self.exchanged.deduct(exchanged_amount);
            self.paid_amount += paid_amount;
            Some(
                Cost::new(
                    paid_amount.neg(),
                    deducted,
                    self.is_vault
                )
            )
        }
    }

    fn add_cash(&mut self, paid_amount: Decimal, amount: Decimal) {
        if let Money::Cash(cash) = &mut self.exchanged {
            cash.amount += amount;
            self.paid_amount += paid_amount;
        }
    }

    fn maybe_deduct_coupon_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Coupon(_), false) => self.maybe_deduct(paid_amount),
            _ => None,
        }
    }

    fn maybe_deduct_cash_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Cash(_), false) => self.maybe_deduct(paid_amount),
            _ => None,
        }
    }

    fn maybe_deduct_vault_coupon_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Coupon(_), true) => self.maybe_deduct(paid_amount),
            _ => None,
        }
    }

    fn maybe_deduct_vault_cash_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Cash(_), true) => self.maybe_deduct(paid_amount),
            _ => None,
        }
    }
}

struct Deductor<'a> {
    costs: &'a mut Vec<Cost>,
    remaining: Decimal,
    result: Vec<Cost>
}

impl<'a> Deductor<'a>
{
    fn new(costs: &mut Vec<Cost>, paid_amount: Decimal) -> Deductor {
        Deductor { costs, remaining: paid_amount, result: vec![] }
    }

    /// Use the given closure to deduct costs from `self.costs`
    fn maybe_deduct<T>(&mut self, deduct_fun: T) -> &mut Deductor<'a>
        where T: Fn(&mut Cost, Decimal) -> Option<Cost>
    {
        if !self.remaining.eq(&dec!(0)) {
            self.costs.iter_mut()
                .rev()
                .fold((self.remaining, &mut self.result), |(remaining, acc), cost| {
                    match remaining.eq(&dec!(0)) {
                        false => {
                            let amount = remaining.max(cost.paid_amount.neg());
                            match deduct_fun(cost, amount) {
                                None =>
                                    (remaining, acc),
                                Some(cost) => {
                                    acc.push(cost);
                                    self.remaining -= amount;
                                    (remaining.sub(amount), acc)
                                }
                            }
                        }
                        true =>
                            (remaining, acc)
                    }
                });
            self.costs.retain(|c| !c.paid_amount.is_zero());
        }
        self
    }

    fn collect(&self) -> Vec<Cost> {
        self.result.clone()
    }
}


#[cfg(test)]
mod test {
    use crate::calculator::{CostBook, Money, TaxableTrade, Direction, Trade};
    use crate::calculator::cost_book::Cost;
    use rust_decimal_macros::dec;
    use std::error::Error;

    #[test]
    fn should_add_buy() -> Result<(), Box<dyn Error>> {
        /*
         * Given
         */
        let mut book = CostBook::new("DOGE".to_string(), "SEK".to_string());

        /*
         * When
         */
        let trade = Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(39.94),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-20),
            date: "2021-11-11 18:03:13".to_string(),
            is_vault: true
        };
        book.add_buy(&trade);

        let trade = Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(2000),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(-5080.60),
            date: "2021-12-31 17:54:48".to_string(),
            is_vault: false
        };
        book.add_buy(&trade);

        let trade = Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(200),
            exchanged_currency: "EOS".to_string(),
            exchanged_amount: dec!(-500),
            date: "2022-02-03 10:30:29".to_string(),
            is_vault: false
        };
        book.add_buy(&trade);

        let trade = Trade {
            direction: Direction::Buy,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(30.3),
            exchanged_currency: "EOS".to_string(),
            exchanged_amount: dec!(-62.35),
            date: "2022-02-04 11:01:35".to_string(),
            is_vault: false
        };
        book.add_buy(&trade);

        /*
         * Then
         */
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

    #[test]
    fn should_add_sell() -> Result<(), Box<dyn Error>> {
        /*
         * Given
         */
        let mut book = CostBook::new("DOGE".to_string(), "SEK".to_string());

        let coupon = Money::new_coupon("EOS".to_string(), dec!(-500), "2021-02-03 10:30:29".to_string());
        book.costs.push(Cost::new(dec!(200), coupon, false));
        let coupon = Money::new_coupon("BTC".to_string(), dec!(-0.0000101), "2021-03-04 11:31:30".to_string());
        book.costs.push(Cost::new(dec!(1000), coupon, false));
        let cash = Money::new_cash("SEK".to_string(), dec!(-21000));
        book.costs.push(Cost::new(dec!(10000), cash, false));
        let cash = Money::new_cash("SEK".to_string(), dec!(-10));
        book.costs.push(Cost::new(dec!(4.5), cash, true));

        /*
         * When
         */
        let trade = Trade {
            direction: Direction::Sell,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(-50),
            exchanged_currency: "SEK".to_string(),
            exchanged_amount: dec!(200.63),
            date: "2022-05-05 05:01:12".to_string(),
            is_vault: false
        };
        let x = book.add_sell(&trade)?;

        /*
         * Then
         */
        assert_eq!(x, TaxableTrade::new(
            Some("2022-05-05 05:01:12".to_string()),
            "DOGE".to_string(),
            dec!(-50),
            Money::new_cash("SEK".to_string(), dec!(200.63)),
            vec![Money::new_cash("SEK".to_string(), dec!(-105))],
            Some(dec!(95.63))
        ));

        let trade = Trade {
            direction: Direction::Sell,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(-50),
            exchanged_currency: "BTC".to_string(),
            exchanged_amount: dec!(0.0000201),
            date: "2022-07-06 06:02:13".to_string(),
            is_vault: false
        };
        let x = book.add_sell(&trade)?;
        assert_eq!(x, TaxableTrade::new(
            Some("2022-07-06 06:02:13".to_string()),
            "DOGE".to_string(),
            dec!(-50),
            Money::new_coupon("BTC".to_string(), dec!(0.0000201), "2022-07-06 06:02:13".to_string()),
            vec![Money::new_coupon("BTC".to_string(), dec!(-0.000000505), "2021-03-04 11:31:30".to_string())],
            None
        ));

        let trade = Trade {
            direction: Direction::Sell,
            paid_currency: "DOGE".to_string(),
            paid_amount: dec!(-1250),
            exchanged_currency: "BCH".to_string(),
            exchanged_amount: dec!(325),
            date: "2022-08-07 07:03:14".to_string(),
            is_vault: false
        };
        let x = book.add_sell(&trade)?;
        assert_eq!(x, TaxableTrade::new(
            Some("2022-08-07 07:03:14".to_string()),
            "DOGE".to_string(),
            dec!(-1250),
            Money::new_coupon("BCH".to_string(), dec!(325), "2022-08-07 07:03:14".to_string()),
            vec![ Money::new_coupon("BTC".to_string(), dec!(-0.000009595), "2021-03-04 11:31:30".to_string())
                  , Money::new_coupon("EOS".to_string(), dec!(-500), "2021-02-03 10:30:29".to_string())
                  , Money::new_cash("SEK".to_string(), dec!(-210))
            ],
            None
        ));

        Ok(())
    }

    #[test]
    fn should_deduct_from_cost() -> Result<(), Box<dyn Error>> {
        let cash = Money::new_cash("SEK".to_string(), dec!(-16000));
        let mut cost = Cost::new(dec!(7500), cash, true);
        let deducted = cost.maybe_deduct(dec!(-500));
        assert_eq!(deducted, Some(Cost{
            paid_amount: dec!(500),
            exchanged: Money::new_cash("SEK".to_string(), dec!(-1066.6666666666666666666666666)),
            is_vault: true
        }));

        let coupon = Money::new_coupon("EOS".to_string(), dec!(-500), "2021-02-03 10:30:29".to_string());
        let mut cost = Cost::new(dec!(200), coupon, false);
        let deducted = cost.maybe_deduct(dec!(-50));
        assert_eq!(deducted, Some(Cost{
            paid_amount: dec!(50),
            exchanged: Money::new_coupon("EOS".to_string(), dec!(-125), "2021-02-03 10:30:29".to_string()),
            is_vault: false
        }));

        Ok(())
    }
}
