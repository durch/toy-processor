#![no_main]
use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use rust_decimal::Decimal;
use std::collections::HashMap;

use toy_processor::account::AccountMap;
use toy_processor::deposit_store::StoredDeposit;
use toy_processor::transactions::{ChargebackTx, DepositTx, DisputeTx, ResolveTx, WithdrawalTx};

// Verified constructors:
// - DepositTx::new(client: u16, id: u32, amount: Decimal)
// - WithdrawalTx::new(client: u16, id: u32, amount: Decimal)
// - DisputeTx::new(client: u16, id: u32)
// - ResolveTx::new(client: u16, id: u32)
// - ChargebackTx::new(client: u16, id: u32)
//
// Verified process() signatures:
// - DepositTx::process(&self, &mut AccountMap, &mut impl DepositStore)
// - WithdrawalTx::process(&self, &mut AccountMap)  <- only accounts!
// - DisputeTx::process(&self, &mut AccountMap, &mut impl DepositStore)
// - ResolveTx::process(&self, &mut AccountMap, &mut impl DepositStore)
// - ChargebackTx::process(&self, &mut AccountMap, &mut impl DepositStore)

#[derive(Debug, Clone)]
enum FuzzTx {
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

impl<'a> Arbitrary<'a> for FuzzTx {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, libfuzzer_sys::arbitrary::Error> {
        // Bias toward smaller IDs for more interesting collisions
        let client: u16 = u.int_in_range(u16::MIN..=u16::MAX)?;
        let tx: u32 = u.int_in_range(u32::MIN..=u32::MAX)?;
        let amount: i64 = u.int_in_range(i64::MIN..=i64::MAX)?;
        let amount = Decimal::new(amount, 4);

        match u.int_in_range(0..=4)? {
            0 => Ok(FuzzTx::Deposit { client, tx, amount }),
            1 => Ok(FuzzTx::Withdrawal { client, tx, amount }),
            2 => Ok(FuzzTx::Dispute { client, tx }),
            3 => Ok(FuzzTx::Resolve { client, tx }),
            _ => Ok(FuzzTx::Chargeback { client, tx }),
        }
    }
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    transactions: Vec<FuzzTx>,
}

fuzz_target!(|input: FuzzInput| {
    let mut accounts = AccountMap::new();
    let mut deposits: HashMap<u32, StoredDeposit> = HashMap::new();

    for ftx in &input.transactions {
        let _ = match ftx {
            FuzzTx::Deposit { client, tx, amount } => {
                DepositTx::new(*client, *tx, *amount).process(&mut accounts, &mut deposits)
            }
            FuzzTx::Withdrawal { client, tx, amount } => {
                WithdrawalTx::new(*client, *tx, *amount).process(&mut accounts)
            }
            FuzzTx::Dispute { client, tx } => {
                DisputeTx::new(*client, *tx).process(&mut accounts, &mut deposits)
            }
            FuzzTx::Resolve { client, tx } => {
                ResolveTx::new(*client, *tx).process(&mut accounts, &mut deposits)
            }
            FuzzTx::Chargeback { client, tx } => {
                ChargebackTx::new(*client, *tx).process(&mut accounts, &mut deposits)
            }
        };
    }
});
