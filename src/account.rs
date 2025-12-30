use std::collections::HashMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::error::Error;

#[derive(Default)]
pub struct AccountMap {
    clients: HashMap<u16, Account>,
}

impl AccountMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    pub fn get_or_create(&mut self, client: u16) -> &mut Account {
        self.clients
            .entry(client)
            .or_insert_with(|| Account::new(client))
    }

    pub fn get_mut(&mut self, client: u16) -> Result<&mut Account, Error> {
        self.clients
            .get_mut(&client)
            .ok_or(Error::AccountNotFound(client))
    }

    pub fn into_iter_sorted(self) -> impl Iterator<Item = Account> {
        let mut accounts: Vec<_> = self.clients.into_values().collect();
        accounts.sort_by_key(|a| a.client);
        accounts.into_iter()
    }

    pub fn merge(&mut self, other: AccountMap) {
        self.clients.extend(other.clients);
    }
}

#[derive(Default, Debug)]
pub struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    locked: bool,
}

#[derive(Serialize)]
pub struct AccountOutput {
    client: u16,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<Account> for AccountOutput {
    fn from(account: Account) -> Self {
        Self {
            client: account.client,
            available: format!("{:.4}", account.available),
            held: format!("{:.4}", account.held),
            total: format!("{:.4}", account.total()),
            locked: account.locked,
        }
    }
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            ..Default::default()
        }
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    #[allow(dead_code)]
    pub fn available(&self) -> Decimal {
        self.available
    }

    #[allow(dead_code)]
    pub fn held(&self) -> Decimal {
        self.held
    }

    pub fn deposit(&mut self, amount: Decimal) -> Result<(), Error> {
        self.throw_locked()?;
        self.available += amount;
        Ok(())
    }

    // Allows available to go negative. This is clawback semantics -
    // if client deposited 100, withdrew 80, then deposit is disputed, we hold the full 100
    // and available becomes -80. The client owes this amount.
    pub fn dispute(&mut self, amount: Decimal) -> Result<(), Error> {
        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), Error> {
        self.throw_locked()?;
        if self.available < amount {
            return Err(Error::InsufficientFunds {
                client: self.client,
                available: self.available,
                requested: amount,
            });
        }
        self.available -= amount;
        Ok(())
    }

    pub fn resolve(&mut self, amount: Decimal) -> Result<(), Error> {
        self.held -= amount;
        self.available += amount;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: Decimal) -> Result<(), Error> {
        self.held -= amount;
        self.locked = true;
        Ok(())
    }

    fn throw_locked(&self) -> Result<(), Error> {
        if self.locked {
            Err(Error::AccountLocked(self.client))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(n: i64) -> Decimal {
        Decimal::new(n, 0)
    }

    #[test]
    fn withdraw_insufficient_funds() {
        let mut account = Account::new(1);
        account.deposit(dec(50)).unwrap();

        let result = account.withdraw(dec(100));

        assert!(matches!(result, Err(Error::InsufficientFunds { .. })));
    }

    #[test]
    fn deposit_on_locked_account() {
        let mut account = Account::new(1);
        account.deposit(dec(100)).unwrap();
        account.dispute(dec(100)).unwrap();
        account.chargeback(dec(100)).unwrap(); // locks account

        let result = account.deposit(dec(50));

        assert!(matches!(result, Err(Error::AccountLocked(1))));
    }

    #[test]
    fn withdraw_on_locked_account() {
        let mut account = Account::new(1);
        account.deposit(dec(100)).unwrap();
        account.dispute(dec(100)).unwrap();
        account.chargeback(dec(100)).unwrap(); // locks account

        let result = account.withdraw(dec(10));

        assert!(matches!(result, Err(Error::AccountLocked(1))));
    }

    #[test]
    fn dispute_on_locked_account_succeeds() {
        let mut account = Account::new(1);
        account.deposit(dec(200)).unwrap();
        account.dispute(dec(100)).unwrap();
        account.chargeback(dec(100)).unwrap(); // locks account, 100 available remains

        // Dispute should still work on locked accounts
        let result = account.dispute(dec(50));

        assert!(result.is_ok());
        assert_eq!(account.available, dec(50));
        assert_eq!(account.held, dec(50));
    }
}
