# Testing the Enshrined DEX Integration

This document explains how to run tests for the enshrined DEX integration with op-rbuilder.

## Test Types

We have three types of tests:

### 1. Unit Tests (DexHandler)
Fast, standalone tests that don't require the full builder infrastructure.

**Location**: `op-rbuilder/crates/op-rbuilder/src/dex/handler.rs`

**Tests**:
- `test_create_pair_with_whitelist` - Tests pair creation with whitelist enforcement
- `test_complete_dex_flow` - Full flow: create pair → place order → get quote → swap
- `test_whitelist_management` - Tests adding/removing addresses from whitelist

**Run**:
```bash
cd op-rbuilder
cargo test --package op-rbuilder --lib dex::handler::tests
```

**Example output**:
```
running 3 tests
test dex::handler::tests::test_create_pair_with_whitelist ... ok
test dex::handler::tests::test_complete_dex_flow ... ok
test dex::handler::tests::test_whitelist_management ... ok
```

### 2. Integration Tests (Test Framework)
Tests that use the full op-rbuilder test framework to send actual transactions.

**Location**: `op-rbuilder/crates/op-rbuilder/src/tests/dex.rs`

**Tests**:
- `dex_create_pair_and_swap` - End-to-end test creating pair and executing swap
- `dex_create_pair_unauthorized` - Tests whitelist enforcement with real transactions
- `dex_get_quote` - Tests getting quotes after setting up liquidity
- `test_dex_handler_create_pair_and_swap` - Unit test with full calldata encoding

**Run**:
```bash
cd op-rbuilder
cargo test --package op-rbuilder --test '*' dex
```

Or for a specific test:
```bash
cd op-rbuilder
cargo test --package op-rbuilder --test '*' dex_create_pair_and_swap
```

### 3. Library Tests (enshrined-dex)
Tests for the underlying DEX library.

**Location**: `crates/dex/tests/`

**Run**:
```bash
cd crates/dex
cargo test
```

## Quick Start

### Run All DEX Tests
```bash
# From project root
cd op-rbuilder
cargo test dex

# This runs both unit tests and integration tests
```

### Run Only Fast Unit Tests
```bash
cd op-rbuilder
cargo test --lib dex
```

### Run With Output
```bash
cd op-rbuilder
cargo test dex -- --nocapture
```

## Test Scenarios Covered

### ✅ Whitelist Enforcement
- ✅ Whitelisted address can create pairs
- ✅ Non-whitelisted address cannot create pairs
- ✅ Whitelist can be modified dynamically

### ✅ DEX Operations
- ✅ Create trading pair
- ✅ Place limit orders
- ✅ Cancel orders
- ✅ Execute swaps with price-time priority
- ✅ Get quotes for trades
- ✅ Multi-hop routing

### ✅ Transaction Flow
- ✅ Transactions sent to predeploy address
- ✅ DEX handler intercepts and processes
- ✅ Receipts generated with correct status
- ✅ Events emitted
- ✅ Gas costs applied

### ✅ Error Handling
- ✅ Unauthorized pair creation fails
- ✅ Invalid calldata rejected
- ✅ Slippage protection works
- ✅ Order not found errors

## Example Test Run

```bash
$ cd op-rbuilder
$ cargo test --lib dex::handler::tests::test_complete_dex_flow -- --nocapture

running 1 test
test dex::handler::tests::test_complete_dex_flow ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Writing New Tests

### Unit Test Template

```rust
#[test]
fn test_my_dex_feature() {
    use alloy_primitives::address;
    use alloy_sol_types::SolValue;

    let whitelisted = address!("0x42...");
    let handler = DexHandler::new_with_whitelist(vec![whitelisted]);

    // Encode calldata
    let calldata = [
        selectors::YOUR_FUNCTION.as_slice(),
        &(param1, param2).abi_encode(),
    ]
    .concat();

    // Execute
    let result = handler
        .handle_transaction(whitelisted, &calldata.into(), U256::ZERO)
        .expect("operation should succeed");

    // Assert
    match result {
        DexResult::YourExpectedResult { .. } => { /* verify */ }
        _ => panic!("unexpected result"),
    }
}
```

### Integration Test Template

```rust
#[rb_test]
async fn my_dex_integration_test(rbuilder: LocalInstance) -> eyre::Result<()> {
    let driver = rbuilder.driver().await?;

    // Create transaction
    let tx = driver
        .create_transaction()
        .from(some_address)
        .to(DEX_PREDEPLOY_ADDRESS)
        .input(calldata)
        .send()
        .await?;

    // Build block
    let block = driver.build_new_block_with_current_timestamp(None).await?;

    // Verify receipt
    let receipt = driver
        .get_transaction_receipt(*tx.tx_hash())
        .await?
        .expect("receipt should exist");

    assert_eq!(receipt.status(), true);
    Ok(())
}
```

## Debugging Tests

### Enable Logging
```bash
RUST_LOG=debug cargo test dex -- --nocapture
```

### Run Single Test
```bash
cargo test --lib dex::handler::tests::test_complete_dex_flow
```

### Run Tests in Series (not parallel)
```bash
cargo test dex -- --test-threads=1
```

## Continuous Integration

For CI/CD pipelines:

```yaml
# .github/workflows/test.yml
name: DEX Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run DEX unit tests
        run: cd op-rbuilder && cargo test --lib dex
      - name: Run DEX integration tests
        run: cd op-rbuilder && cargo test dex
```

## Performance Benchmarks

To benchmark DEX operations:

```bash
cd crates/dex
cargo bench
```

## Common Issues

### Issue: Tests fail with "dex module not found"
**Solution**: Make sure you're in the `op-rbuilder` directory
```bash
cd op-rbuilder
cargo test dex
```

### Issue: Integration tests timeout
**Solution**: Increase timeout or run in release mode
```bash
cargo test --release dex
```

### Issue: "Address not whitelisted" in integration tests
**Solution**: Use `driver.get_builder_address()` to get the whitelisted address
```rust
let whitelisted = driver.get_builder_address().await?;
```

## Test Coverage

To generate coverage reports:

```bash
cargo install cargo-tarpaulin
cd op-rbuilder
cargo tarpaulin --lib --out Html --output-dir coverage -- dex
# Open coverage/index.html
```

## Next Steps

After tests pass:
1. Run the full op-rbuilder test suite to ensure no regressions
2. Test with high transaction volume
3. Profile performance under load
4. Add fuzzing tests for edge cases
5. Test state persistence and recovery

## Questions?

Check:
- `ENSHRINED_DEX_INTEGRATION.md` - Integration architecture
- `INTEGRATION_COMPLETE.md` - Complete setup guide
- `crates/dex/CLAUDE.md` - DEX library documentation
