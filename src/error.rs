use crate::deposit_store::DepositStateError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Usage: cargo run -- <transactions.csv>")]
    MissingArgument,

    #[error("Account {0} is locked")]
    AccountLocked(u16),

    #[error("Account {0} not found")]
    AccountNotFound(u16),

    #[error("Insufficient funds for client {client}: available {available}, requested {requested}")]
    InsufficientFunds {
        client: u16,
        available: rust_decimal::Decimal,
        requested: rust_decimal::Decimal,
    },

    #[error("Client mismatch for transaction {tx_id}: expected {expected}, found {found}")]
    ClientMismatch {
        tx_id: u32,
        expected: u16,
        found: u16,
    },

    #[error("Stored deposit {0} not found")]
    StoredDepositNotFound(u32),

    #[error("Deposit state error: {0}")]
    DepositState(#[from] DepositStateError),

    #[error("Invalid transaction row: {0}")]
    InvalidTransactionRow(u32),
}
