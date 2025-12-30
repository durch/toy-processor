use rust_decimal::Decimal;
use serde::Deserialize;

mod chargeback_tx;
mod deposit_tx;
mod dispute_tx;
mod resolve_tx;
mod withdrawal_tx;

pub use chargeback_tx::ChargebackTx;
pub use deposit_tx::DepositTx;
pub use dispute_tx::DisputeTx;
pub use resolve_tx::ResolveTx;
pub use withdrawal_tx::WithdrawalTx;

use crate::error::Error;

#[derive(Debug, Deserialize)]
pub struct TransactionRow {
    #[serde(rename = "type")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

impl TransactionRow {
    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn tx(&self) -> u32 {
        self.tx
    }

    pub fn tx_type(&self) -> &str {
        &self.tx_type
    }

    pub fn amount(&self) -> Option<Decimal> {
        self.amount
    }

    pub fn should_dedupe(&self) -> bool {
        matches!(self.tx_type.as_str(), "deposit" | "withdrawal")
    }
}

#[derive(Debug)]
pub enum Transaction {
    Deposit(DepositTx),
    Withdrawal(WithdrawalTx),
    Dispute(DisputeTx),
    Resolve(ResolveTx),
    Chargeback(ChargebackTx),
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = Error;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        match row.tx_type.as_str() {
            "deposit" => {
                if let Some(amount) = row.amount {
                    // Arguably this could be <= 0, but there might be a special case where 0 value deposits and withdrawals are valid,
                    // opens up a spam venue but feels like that should be handled at client level if needed.
                    if amount.is_sign_negative() {
                        return Err(Error::InvalidTransactionRow(row.tx()));
                    }
                    let amount = amount.round_dp(4);
                    Ok(Transaction::Deposit(DepositTx::new(
                        row.client, row.tx, amount,
                    )))
                } else {
                    Err(Error::InvalidTransactionRow(row.tx))
                }
            }
            "withdrawal" => {
                if let Some(amount) = row.amount {
                    if amount.is_sign_negative() {
                        return Err(Error::InvalidTransactionRow(row.tx()));
                    }
                    let amount = amount.round_dp(4);
                    Ok(Transaction::Withdrawal(WithdrawalTx::new(
                        row.client, row.tx, amount,
                    )))
                } else {
                    Err(Error::InvalidTransactionRow(row.tx))
                }
            }
            "dispute" => Ok(Transaction::Dispute(DisputeTx::new(row.client, row.tx))),
            "resolve" => Ok(Transaction::Resolve(ResolveTx::new(row.client, row.tx))),
            "chargeback" => Ok(Transaction::Chargeback(ChargebackTx::new(
                row.client, row.tx,
            ))),
            _ => Err(Error::InvalidTransactionRow(row.tx)),
        }
    }
}
