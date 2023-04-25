mod cost_book;
mod money;
mod taxable_trade;
pub(crate) mod trade;
pub(crate) type Currency = String;
pub(crate) use crate::calculator::cost_book::taxable_trades;
pub(crate) use crate::calculator::taxable_trade::TaxableTrade;
pub(crate) use crate::calculator::money::Money;