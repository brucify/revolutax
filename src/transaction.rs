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
}

#[derive(Debug)]
pub(crate) enum TransactionType {
    Buy,
    Sell
}

pub(crate) type Currency = String;
