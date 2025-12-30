use crate::{account::AccountMap, deposit_store::DepositStore, error::Error};

#[derive(Debug)]
pub struct ChargebackTx {
    client: u16,
    id: u32,
}

impl ChargebackTx {
    pub fn new(client: u16, id: u32) -> Self {
        Self { client, id }
    }

    fn client(&self) -> u16 {
        self.client
    }

    fn id(&self) -> u32 {
        self.id
    }

    // State transition (set_chargedback) is the idempotency guard. The deposit state machine
    // rejects invalid transitions (AlreadyChargedback, etc.), preventing double-processing.
    pub fn process(
        &self,
        accounts: &mut AccountMap,
        stored_deposits: &mut impl DepositStore,
    ) -> Result<(), Error> {
        if let Some(stored_deposit) = stored_deposits.get_mut(self.id()) {
            stored_deposit.ensure_client_matches(self.id(), self.client())?;
            let account = accounts.get_mut(self.client())?;
            stored_deposit.set_chargedback()?;
            account.chargeback(stored_deposit.amount())?;

            Ok(())
        } else {
            Err(Error::StoredDepositNotFound(self.id()))
        }
    }
}
