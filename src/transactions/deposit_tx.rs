use crate::{account::AccountMap, deposit_store::DepositStore, error::Error};
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct DepositTx {
    client: u16,
    id: u32,
    amount: Decimal,
}

impl DepositTx {
    pub fn new(client: u16, id: u32, amount: Decimal) -> Self {
        Self { client, id, amount }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn process(
        &self,
        accounts: &mut AccountMap,
        stored_deposits: &mut impl DepositStore,
    ) -> Result<(), Error> {
        let account = accounts.get_or_create(self.client());
        account.deposit(self.amount())?;
        stored_deposits.insert(self);
        Ok(())
    }
}
