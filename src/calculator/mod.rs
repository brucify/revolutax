mod cost_book;
pub(crate) mod money;
pub(crate) mod taxable_trade;
pub(crate) mod trade;
pub(crate) type Currency = String;
pub(crate) use crate::calculator::cost_book::taxable_trades;
pub(crate) use crate::calculator::cost_book::all_taxable_trades;
pub(crate) use crate::calculator::taxable_trade::TaxableTrade;