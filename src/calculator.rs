use crate::trade::{Currency, Trade, Direction, Money};
use log::debug;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt::Debug;
use std::io;
use std::ops::{Neg, Sub};

// 1. Bought Crypto 1 from SEK      (cost in SEK),  sold to SEK      (sales in SEK)
// 2. Bought Crypto 1 from SEK      (cost in SEK),  sold to Crypto 2 (SEK price as sales)
// 3. Bought from Crypto 2 (SEK price as cost),     sold to Crypto 3 (SEK price as sales)
// 4. Bought from Crypto 3 (SEK price as cost),     sold to SEK      (sales in SEK)
#[derive(Debug, PartialEq)]
pub(crate) struct TaxableTrade {
    date: String,
    currency: Currency,             // Valutakod
    amount: Decimal,                // Antal
    income: Money,                  // Försäljningspris
    costs: Vec<Money>,              // Omkostnadsbelopp
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

#[derive(Debug, PartialEq, Clone)]
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
        if self.paid_amount + paid_amount < dec!(0) {
            None
        } else {
            let exchanged_amount = self.exchanged.amount() / self.paid_amount * paid_amount.abs();
            let deducted = self.exchanged.deduct(exchanged_amount);
            self.paid_amount += paid_amount;
            let deducted_cost = Cost::new(paid_amount.neg(), deducted, self.is_vault);
            Some(deducted_cost)
        }
    }

    fn add_cash(&mut self, paid_amount: Decimal, amount: Decimal) {
        if let Money::Cash(cash) = &mut self.exchanged {
            cash.amount += amount;
            self.paid_amount += paid_amount;
        }
    }

    fn deduct_coupon_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Coupon(_), false) => self.deduct(paid_amount),
            _ => None,
        }
    }

    fn deduct_cash_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Cash(_), false) => self.deduct(paid_amount),
            _ => None,
        }
    }

    fn deduct_vault_coupon_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Coupon(_), true) => self.deduct(paid_amount),
            _ => None,
        }
    }

    fn deduct_vault_cash_cost(&mut self, paid_amount: Decimal) -> Option<Cost> {
        match (&self.exchanged, self.is_vault) {
            (Money::Cash(_), true) => self.deduct(paid_amount),
            _ => None,
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
        CostBook {
            base,
            currency,
            costs: vec![],
        }
    }
    fn add_buy(&mut self, transaction: &Trade) {
        match transaction.to_money(&self.base) {
            Money::Cash(cash) => {
                self.find_cash_cost_mut(transaction.is_vault).map(|cost|
                    cost.add_cash(transaction.paid_amount, cash.amount)
                );
            }
            income @ Money::Coupon(_) => {
                let coupon_cost = Cost::new(transaction.paid_amount, income, transaction.is_vault);
                self.costs.push(coupon_cost);
            }
        }
    }

    fn add_sell(&mut self, transaction: &Trade) -> io::Result<TaxableTrade> {
        let income = transaction.to_money(&self.base);
        let costs =
            self.find_and_deduct_cost(&income, transaction.paid_amount)?
                .into_iter()
                .map(|c| c.exchanged)
                .collect();
        let net_income = income.to_net_income(&costs);
        Ok(TaxableTrade {
            date: transaction.date.clone(),
            currency: transaction.paid_currency.clone(),
            amount: transaction.paid_amount,
            income,
            costs,
            net_income
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

    /// Find the costs for the given `income`. Then deduct them from the book.
    /// If `income` is `Money::Cash`, try deduct from the cash in the `CostBook`.
    /// Likewise, if `income` is `Money::Coupon`, try deduct from the coupons in the `CostBook`.
    /// Only start deducting from the vault if there are no non-vault costs to deduct.
    /// Returns a `Vec<Cost>` which is a list of deducted costs.
    fn find_and_deduct_cost(&mut self, income: &Money, paid_amount: Decimal) -> io::Result<Vec<Cost>> {
        let mut ddr = Deductor::new(&mut self.costs, paid_amount);
        let deducted =
            match income {
                Money::Cash(_) =>
                    ddr.deduct(Cost::deduct_cash_cost)
                        .deduct(Cost::deduct_coupon_cost)
                        .deduct(Cost::deduct_vault_cash_cost)
                        .deduct(Cost::deduct_vault_coupon_cost)
                        .collect(),
                Money::Coupon(_) =>
                    ddr.deduct(Cost::deduct_coupon_cost)
                        .deduct(Cost::deduct_cash_cost)
                        .deduct(Cost::deduct_vault_coupon_cost)
                        .deduct(Cost::deduct_vault_cash_cost)
                        .collect(),
            };

        match ddr.remaining.eq(&dec!(0)) {
            true => Ok(deducted),
            false => Err(io::Error::from(io::ErrorKind::InvalidData)) // Not enough costs to deduct from
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
    fn deduct<T>(&mut self, deduct: T) -> &mut Deductor<'a>
        where T: Fn(&mut Cost, Decimal) -> Option<Cost>
    {
        if !self.remaining.eq(&dec!(0)) {
            self.costs.iter_mut().rev().fold((self.remaining, &mut self.result), |(remaining, acc), cost| {
                if remaining.eq(&dec!(0)) { (remaining, acc) } else {
                    let amount = remaining.max(cost.paid_amount.neg());

                    match deduct(cost, amount) {
                        None => (remaining, acc),
                        Some(cost) => {
                            acc.push(cost);
                            self.remaining -= amount;
                            (remaining.sub(amount), acc)
                        }
                    }
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

pub(crate) async fn tax(trades: &Vec<Trade>, currency: &Currency, base: &Currency) -> io::Result<Vec<TaxableTrade>> {
    let book = CostBook::new(currency.clone(), base.clone());
    let (trades, b) =
        trades.iter().fold((vec![], book), |(mut acc, mut book), t| {
            match t.direction {
                Direction::Buy => book.add_buy(t),
                Direction::Sell => {
                    let x = book.add_sell(t).unwrap();
                    acc.push(x);
                },
            }
            (acc, book)
        });
    debug!("Remaining costs for {:?}:", b.currency);
    b.costs.iter().for_each(|c| debug!("{:?}", c));
    debug!("Taxable transactions:");
    trades.iter().for_each(|t| debug!("{:?}", t));
    Ok(trades)
}

#[cfg(test)]
mod test {
    use crate::calculator::{Cost, CostBook, TaxableTrade};
    use crate::trade::{Cash, Coupon, Money, Trade, Direction};
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
        assert_eq!(x, TaxableTrade {
            date: "2022-05-05 05:01:12".to_string(),
            currency: "DOGE".to_string(),
            amount: dec!(-50),
            income: Money::Cash(Cash{ currency: "SEK".to_string(), amount: dec!(200.63) }),
            costs: vec![Money::Cash(Cash{ currency: "SEK".to_string(), amount: dec!(-105) })],
            net_income: Some(dec!(95.63))
        });

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
        assert_eq!(x, TaxableTrade {
            date: "2022-07-06 06:02:13".to_string(),
            currency: "DOGE".to_string(),
            amount: dec!(-50),
            income: Money::Coupon(Coupon{ currency: "BTC".to_string(), amount: dec!(0.0000201), date: "2022-07-06 06:02:13".to_string() }),
            costs: vec![Money::Coupon(Coupon{ currency: "BTC".to_string(), amount: dec!(-0.000000505), date: "2021-03-04 11:31:30".to_string() })],
            net_income: None
        });

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
        assert_eq!(x, TaxableTrade {
            date: "2022-08-07 07:03:14".to_string(),
            currency: "DOGE".to_string(),
            amount: dec!(-1250),
            income: Money::Coupon(Coupon{ currency: "BCH".to_string(), amount: dec!(325), date: "2022-08-07 07:03:14".to_string() }),
            costs: vec![ Money::Coupon(Coupon{ currency: "BTC".to_string(), amount: dec!(-0.000009595), date: "2021-03-04 11:31:30".to_string() })
                       , Money::Coupon(Coupon{ currency: "EOS".to_string(), amount: dec!(-500), date: "2021-02-03 10:30:29".to_string() })
                       , Money::Cash(Cash{ currency: "SEK".to_string(), amount: dec!(-210) })
                       ],
            net_income: None
        });

        Ok(())
    }

    #[test]
    fn should_deduct_from_cost() -> Result<(), Box<dyn Error>> {
        let cash = Money::new_cash("SEK".to_string(), dec!(-16000));
        let mut cost = Cost::new(dec!(7500), cash, true);
        let deducted = cost.deduct(dec!(-500));
        assert_eq!(deducted, Some(Cost{
            paid_amount: dec!(500),
            exchanged: Money::Cash(Cash{ currency: "SEK".to_string(), amount: dec!(-1066.6666666666666666666666666) }),
            is_vault: true
        }));

        let coupon = Money::new_coupon("EOS".to_string(), dec!(-500), "2021-02-03 10:30:29".to_string());
        let mut cost = Cost::new(dec!(200), coupon, false);
        let deducted = cost.deduct(dec!(-50));
        assert_eq!(deducted, Some(Cost{
            paid_amount: dec!(50),
            exchanged: Money::Coupon(Coupon{ currency: "EOS".to_string(), amount: dec!(-125), date: "2021-02-03 10:30:29".to_string()}),
            is_vault: false
        }));

        Ok(())
    }
}
