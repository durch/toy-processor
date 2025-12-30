use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::transactions::DepositTx;

// Memory scales with deposit count (~20 bytes each). At scale (billions of txs),
// this is impractical. Production alternatives: external storage (DB/Redis), time-based
// dispute windows (e.g., 90 days), or transaction-count limits with cold storage archival.
pub trait DepositStore {
    fn insert(&mut self, tx: &DepositTx);
    #[allow(dead_code)]
    fn get(&self, tx_id: u32) -> Option<&StoredDeposit>;
    fn get_mut(&mut self, tx_id: u32) -> Option<&mut StoredDeposit>;
    #[allow(dead_code)]
    fn remove(&mut self, tx_id: u32) -> Option<StoredDeposit>;
}

impl DepositStore for HashMap<u32, StoredDeposit> {
    fn insert(&mut self, tx: &DepositTx) {
        let stored_deposit = StoredDeposit::from(tx);
        self.insert(tx.id(), stored_deposit);
    }

    fn get(&self, tx_id: u32) -> Option<&StoredDeposit> {
        self.get(&tx_id)
    }

    fn get_mut(&mut self, tx_id: u32) -> Option<&mut StoredDeposit> {
        self.get_mut(&tx_id)
    }

    fn remove(&mut self, tx_id: u32) -> Option<StoredDeposit> {
        self.remove(&tx_id)
    }
}

#[derive(Debug)]
pub struct StoredDeposit {
    client: u16,
    amount: Decimal,
    status: DepositStatus,
}

impl StoredDeposit {
    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn set_disputed(&mut self) -> Result<(), DepositStateError> {
        self.status.dispute()
    }

    pub fn set_resolved(&mut self) -> Result<(), DepositStateError> {
        self.status.resolve()
    }

    pub fn set_chargedback(&mut self) -> Result<(), DepositStateError> {
        self.status.chargeback()
    }

    pub fn ensure_client_matches(
        &self,
        tx_id: u32,
        tx_client: u16,
    ) -> Result<(), crate::error::Error> {
        if tx_client != self.client() {
            Err(crate::error::Error::ClientMismatch {
                tx_id,
                expected: self.client(),
                found: tx_client,
            })
        } else {
            Ok(())
        }
    }
}

impl From<&DepositTx> for StoredDeposit {
    fn from(tx: &DepositTx) -> Self {
        StoredDeposit {
            client: tx.client(),
            amount: tx.amount(),
            status: DepositStatus::Clear,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DepositStatus {
    Clear,
    Disputed,
    Resolved,
    Chargedback,
}

#[derive(thiserror::Error, Debug)]
pub enum DepositStateError {
    // Dispute errors
    #[error("Deposit is already under dispute")]
    AlreadyDisputed,
    #[error("Cannot dispute a resolved deposit")]
    CannotDisputeResolved,
    #[error("Cannot dispute a chargedback deposit")]
    CannotDisputeChargedback,

    // Resolve errors
    #[error("Cannot resolve an undisputed deposit")]
    CannotResolveUndisputed,
    #[error("Deposit has already been resolved")]
    AlreadyResolved,
    #[error("Cannot resolve a chargedback deposit")]
    CannotResolveChargedback,

    // Chargeback errors
    #[error("Cannot chargeback an undisputed deposit")]
    CannotChargebackUndisputed,
    #[error("Cannot chargeback a resolved deposit")]
    CannotChargebackResolved,
    #[error("Deposit has already been chargedback")]
    AlreadyChargedback,
}

impl DepositStatus {
    fn dispute(&mut self) -> Result<(), DepositStateError> {
        match self {
            DepositStatus::Clear => {
                *self = DepositStatus::Disputed;
                Ok(())
            }
            DepositStatus::Disputed => Err(DepositStateError::AlreadyDisputed),
            DepositStatus::Resolved => Err(DepositStateError::CannotDisputeResolved),
            DepositStatus::Chargedback => Err(DepositStateError::CannotDisputeChargedback),
        }
    }

    fn resolve(&mut self) -> Result<(), DepositStateError> {
        match self {
            DepositStatus::Disputed => {
                *self = DepositStatus::Resolved;
                Ok(())
            }
            DepositStatus::Clear => Err(DepositStateError::CannotResolveUndisputed),
            DepositStatus::Resolved => Err(DepositStateError::AlreadyResolved),
            DepositStatus::Chargedback => Err(DepositStateError::CannotResolveChargedback),
        }
    }

    fn chargeback(&mut self) -> Result<(), DepositStateError> {
        match self {
            DepositStatus::Disputed => {
                *self = DepositStatus::Chargedback;
                Ok(())
            }
            DepositStatus::Clear => Err(DepositStateError::CannotChargebackUndisputed),
            DepositStatus::Resolved => Err(DepositStateError::CannotChargebackResolved),
            DepositStatus::Chargedback => Err(DepositStateError::AlreadyChargedback),
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn client_mismatch_rejected() {
        let deposit = StoredDeposit {
            client: 1,
            amount: Decimal::new(100, 0),
            status: DepositStatus::Clear,
        };

        let result = deposit.ensure_client_matches(42, 2); // tx 42, wrong client 2

        assert!(matches!(
            result,
            Err(crate::error::Error::ClientMismatch {
                tx_id: 42,
                expected: 1,
                found: 2
            })
        ));
    }
}
