# Toy Payments Engine

A simple transaction processor that reads CSV transactions, updates client accounts, handles disputes and chargebacks, and outputs final account states.

## Usage

```bash
cargo run --release transactions.csv > accounts.csv
```

## Architecture

### Threading Model

The engine uses a multi-threaded architecture with 4 worker threads. Transactions are partitioned by `client_id % 4`, ensuring all transactions for a single client are processed sequentially by the same worker. This enables parallel processing while maintaining per-client ordering guarantees.

### Deposit Storage

Deposits must be stored for later dispute resolution. Storage is abstracted behind the `DepositStore` trait:

```rust
pub trait DepositStore {
    fn insert(&mut self, tx: &DepositTx);
    fn get(&self, tx_id: u32) -> Option<&StoredDeposit>;
    fn get_mut(&mut self, tx_id: u32) -> Option<&mut StoredDeposit>;
    fn remove(&mut self, tx_id: u32) -> Option<StoredDeposit>;
}
```

Transaction processors are generic over `impl DepositStore`, so swapping to Redis, PostgreSQL, or any other backend requires only implementing this trait.

**Current implementation**: In-memory `HashMap<u32, StoredDeposit>` (~20 bytes per deposit). At scale (billions of transactions), this becomes impractical, hence the trait abstraction.

### Streaming & Deduplication

- **Streaming**: CSV rows are processed one at a time.
- **Bloom Filter**: Transaction (deposits and withdrawals) deduplication uses a bloom filter (0.001% false positive rate). At 10M transactions, uses ~30MB RAM with ~100 potential false drops. At present drops are logged, and while even that is enough for later replication, a separate queue would be more robust.

### Deposit State Machine

```
Clear ──dispute──► Disputed ──resolve──► Resolved
                      │
                      └──chargeback──► Chargedback (account locked)
```

State transitions are enforced by the type system. Invalid transitions (e.g., resolving an undisputed deposit) are rejected.

## Features

| Requirement | Status |
|------------|--------|
| CLI interface `cargo run -- file.csv > output.csv` | OK |
| CSV input parsing (type, client, tx, amount) | OK |
| Whitespace handling | OK |
| 4 decimal precision | OK |
| Deposit increases available/total | OK |
| Withdrawal decreases available/total | OK |
| Withdrawal fails on insufficient funds | OK |
| Dispute: available -, held +, total same | OK |
| Resolve: held -, available +, total same | OK |
| Chargeback: held -, total -, account locked | OK |
| Ignore dispute if tx doesn't exist | OK |
| Ignore resolve if tx doesn't exist/not disputed | OK |
| Ignore chargeback if tx doesn't exist/not disputed | OK |
| Output format (client, available, held, total, locked) | OK |

### Design Decisions

#### 1. Only deposits are disputable

Disputes handle *incoming* fraud (stolen card, reversed ACH) where funds are still in the system and can be held. Withdrawals are *outgoing* - once funds leave, there's nothing to hold or chargeback. Compromised accounts are security incidents (freeze + investigate), not payment disputes. 

#### 2. Negative available balance (clawback semantics)

When a deposit is disputed after partial withdrawal, available can go negative:
- Deposit 100, withdraw 80, dispute deposit → available=-80, held=100, total=20

This is intentional. Without clawback, fraudsters could deposit, withdraw, and avoid dispute. The negative balance represents debt owed.

#### 3. Client mismatch validation

Disputes/resolves/chargebacks are rejected if the client ID doesn't match the original deposit's client. This prevents clients disputing other client transactions.

#### 4. Re-disputing resolved deposits

Once a deposit is resolved, it cannot be disputed again. The state machine enforces: `Resolved → (terminal)`. This prevents dispute loops, and spam. 

#### 5. Disputes on locked accounts

Deposits and withdrawals are blocked on locked accounts, but disputes/resolves can still be processed. This allows resolving existing disputes after a chargeback.

#### 6. Zero-amount transactions

Accepted (no-op effectively). Zero-amounts could have legitimate uses like account verification.

#### 7. Bloom filter trade-off

Transaction deduplication uses a probabilistic bloom filter. At 10M transactions, ~100 valid transactions may be incorrectly dropped as duplicates. This is a space/accuracy trade-off documented in code.

## Testing

```bash
# Unit tests
cargo test

# Integration tests with fixtures
cargo test --test integration

# Fuzz testing (requires nightly)
cargo +nightly fuzz run transaction_processor
```

### Test Fixtures

| Fixture | Description |
|---------|-------------|
| `basic_deposit_withdraw` | Simple deposit/withdrawal flow |
| `dispute_resolve` | Dispute then resolve returns funds |
| `dispute_chargeback` | Dispute then chargeback locks account |
| `locked_account_rejects` | Locked accounts reject deposits/withdrawals |
| `insufficient_funds` | Withdrawal exceeding balance rejected |
| `dispute_nonexistent` | Disputing missing tx ignored |
| `negative_balance_clawback` | Clawback semantics test |
| `double_dispute` | Second dispute on same tx rejected |
| `precision` | 4 decimal place precision |
| `whitespace` | Handles whitespace in CSV |
| `zero_amount` | Zero amounts accepted |
| `negative_amount` | Negative amounts rejected |

## Error Handling

Errors are logged to stderr but don't halt processing. Invalid transactions are skipped, allowing the engine to process the rest of the file.

## Dependencies

- `csv` - CSV parsing
- `rust_decimal` - Precise decimal arithmetic (no floating point errors)
- `serde` - Serialization/deserialization
- `bloomfilter` - Probabilistic deduplication
- `thiserror` - Error handling
- `log` / `env_logger` - Logging
