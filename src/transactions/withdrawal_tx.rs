use crate::{account::AccountMap, error::Error};
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct WithdrawalTx {
    client: u16,
    #[allow(dead_code)]
    id: u32,
    amount: Decimal,
}

impl WithdrawalTx {
    pub fn new(client: u16, id: u32, amount: Decimal) -> Self {
        Self { client, id, amount }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    #[allow(dead_code)]
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn process(&self, accounts: &mut AccountMap) -> Result<(), Error> {
        let account = accounts.get_or_create(self.client());
        account.withdraw(self.amount())?;
        Ok(())
    }
}
