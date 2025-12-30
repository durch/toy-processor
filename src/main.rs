use std::collections::HashMap;
use std::fs::File;
use std::sync::mpsc::{self, Receiver};
use std::{env, thread};

use bloomfilter::Bloom;
use log::{debug, error, info, warn};

use crate::account::{AccountMap, AccountOutput};
use crate::deposit_store::StoredDeposit;
use crate::transactions::{Transaction, TransactionRow};

mod account;
mod deposit_store;
mod error;
mod transactions;

const WORKER_COUNT: usize = 4;
// Roughly ~24 bits per element at the below fp rate, tweakable depending on real world requirements,
// 10 million expected deposit and withdraw txs uses ~30MB RAM, would produce ~100 false positives
const EXPECTED_N_TRANSACTIONS: usize = 10_000_000;
const BLOOM_FP_RATE: f64 = 0.00001;

fn worker_loop(rx: Receiver<TransactionRow>) -> AccountMap {
    let mut accounts = AccountMap::new();
    let mut deposits: HashMap<u32, StoredDeposit> = HashMap::new();

    // Blocks until message or channel closed (sender dropped)
    while let Ok(row) = rx.recv() {
        let transaction: Transaction = match row.try_into() {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to convert transaction: {}", e);
                continue;
            }
        };

        debug!("Processing: {:?}", transaction);

        let result = match &transaction {
            Transaction::Deposit(t) => t.process(&mut accounts, &mut deposits),
            Transaction::Withdrawal(t) => t.process(&mut accounts),
            Transaction::Dispute(t) => t.process(&mut accounts, &mut deposits),
            Transaction::Resolve(t) => t.process(&mut accounts, &mut deposits),
            Transaction::Chargeback(t) => t.process(&mut accounts, &mut deposits),
        };

        if let Err(e) = result {
            error!("Transaction failed: {}", e);
        }
    }

    accounts
}

fn main() -> Result<(), error::Error> {
    env_logger::init();

    let path = env::args().nth(1).ok_or(error::Error::MissingArgument)?;
    info!("Processing transactions from: {}", path);

    let file = File::open(&path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut bloom = Bloom::new_for_fp_rate(EXPECTED_N_TRANSACTIONS, BLOOM_FP_RATE).unwrap();

    let (senders, receivers): (Vec<_>, Vec<_>) = (0..WORKER_COUNT)
        .map(|_| mpsc::channel::<TransactionRow>())
        .unzip();

    let handles: Vec<_> = receivers
        .into_iter()
        .map(|rx| thread::spawn(move || worker_loop(rx)))
        .collect();

    for result in rdr.deserialize() {
        let row: TransactionRow = match result {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse CSV row: {}", e);
                continue;
            }
        };

        if row.should_dedupe() {
            if !bloom.check(&row.tx()) {
                bloom.set(&row.tx());
            } else {
                warn!(
                    "Possible duplicate tx={} client={} type={} amount={:?} - dropped",
                    row.tx(),
                    row.client(),
                    row.tx_type(),
                    row.amount()
                );
                continue;
            }
        }

        let worker_idx = row.client() as usize % WORKER_COUNT;
        {
            let sender = &senders[worker_idx];
            if let Err(e) = sender.send(row) {
                error!("Failed to send transaction to worker {}: {}", worker_idx, e);
            }
        }
    }

    // Explicit drop to avoid another closure and a dedicated thread
    drop(senders);

    let accounts: AccountMap = handles
        .into_iter()
        .filter_map(|h| match h.join() {
            Ok(acc) => Some(acc),
            Err(_) => {
                error!("Worker thread panicked");
                None
            }
        })
        .fold(AccountMap::new(), |mut merged, shard| {
            merged.merge(shard);
            merged
        });

    info!("Processing complete. {} accounts.", accounts.len());

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for account in accounts.into_iter_sorted() {
        wtr.serialize(AccountOutput::from(account))?;
    }
    wtr.flush()?;

    Ok(())
}
