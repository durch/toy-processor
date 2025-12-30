use std::process::Command;

fn run_test(fixture: &str, expected: &str) {
    let output = Command::new("./target/debug/toy-processor")
        .arg(format!("tests/fixtures/{}.csv", fixture))
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), expected.trim(), "Fixture: {}", fixture);
}

#[test]
fn basic_deposit_withdraw() {
    run_test(
        "basic_deposit_withdraw",
        "client,available,held,total,locked
1,85.0000,0.0000,85.0000,false
2,50.0000,0.0000,50.0000,false",
    );
}

#[test]
fn whitespace_handling() {
    run_test(
        "whitespace",
        "client,available,held,total,locked
1,50.5000,0.0000,50.5000,false",
    );
}

#[test]
fn precision_4_decimals() {
    run_test(
        "precision",
        "client,available,held,total,locked
1,15.1235,0.0000,15.1235,false",
    );
}

#[test]
fn dispute_then_resolve_returns_funds() {
    run_test(
        "dispute_resolve",
        "client,available,held,total,locked
1,100.0000,0.0000,100.0000,false",
    );
}

#[test]
fn dispute_then_chargeback_locks_account() {
    run_test(
        "dispute_chargeback",
        "client,available,held,total,locked
1,50.0000,0.0000,50.0000,true",
    );
}

#[test]
fn locked_account_rejects_deposit() {
    // After chargeback, account is locked - subsequent deposit should be rejected
    // Account should still show 0 (the chargebacked amount is gone, new deposit rejected)
    run_test(
        "locked_account_rejects",
        "client,available,held,total,locked
1,0.0000,0.0000,0.0000,true",
    );
}

#[test]
fn insufficient_funds_rejected() {
    // Withdrawal exceeding available balance should be rejected
    // Account should still have original 50
    run_test(
        "insufficient_funds",
        "client,available,held,total,locked
1,50.0000,0.0000,50.0000,false",
    );
}

#[test]
fn dispute_nonexistent_tx_ignored() {
    // Disputing a tx that doesn't exist should be ignored (logged as error)
    // Account should still have original deposit
    run_test(
        "dispute_nonexistent",
        "client,available,held,total,locked
1,100.0000,0.0000,100.0000,false",
    );
}

#[test]
fn negative_balance_clawback() {
    // Deposit 100, withdraw 80, dispute the deposit
    // Available goes negative (-80), held = 100, total = 20
    // This is intentional clawback semantics
    run_test(
        "negative_balance_clawback",
        "client,available,held,total,locked
1,-80.0000,100.0000,20.0000,false",
    );
}

#[test]
fn double_dispute_idempotent() {
    // Disputing same tx twice - second dispute should be rejected by state machine
    // Account should show single dispute: available=0, held=100
    run_test(
        "double_dispute",
        "client,available,held,total,locked
1,0.0000,100.0000,100.0000,false",
    );
}

#[test]
fn zero_amount_transactions() {
    // Zero amount deposit/withdrawal are accepted (no-op effectively)
    run_test(
        "zero_amount",
        "client,available,held,total,locked
1,100.0000,0.0000,100.0000,false",
    );
}

#[test]
fn negative_amount_rejected() {
    // Negative amounts should be rejected - only the valid 100 deposit should process
    run_test(
        "negative_amount",
        "client,available,held,total,locked
1,100.0000,0.0000,100.0000,false",
    );
}
