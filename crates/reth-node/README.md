# Reth Node with Enshrined DEX

Custom Optimism-compatible Reth node with an enshrined DEX implementation.

## Overview

This is a standalone Optimism node built using Reth as an SDK. It replaces the default payload builder with a custom implementation that:

1. Intercepts transactions sent to the DEX predeploy address (`0x4200000000000000000000000000000000000042`)
2. Executes them through the in-memory DEX library (`../dex`)
3. Properly handles gas calculation and receipt generation
4. Maintains DEX state across blocks

## Architecture

- **main.rs**: Node setup and payload builder registration
- **generator.rs**: Payload job generator (creates jobs for each block)
- **job.rs**: Payload job implementation (handles DEX transaction execution)

The DEX state is maintained in a shared `Arc<RwLock<PoolManager>>` that persists across all payload jobs.

## Running the Node

```bash
# Basic node startup
cargo run -p reth-node -- node

# With dev mode (for testing)
cargo run -p reth-node -- node --dev

# With custom datadir
cargo run -p reth-node -- node --datadir ./data

# Enable debug logging
RUST_LOG=debug cargo run -p reth-node -- node
```

## DEX Predeploy Address

The DEX is available at the predeploy address: `0x4200000000000000000000000000000000000042`

## Supported DEX Functions

- `createPair(address,address)` - Create a new trading pair
- `placeLimitOrder(address,address,bool,uint256,uint256,uint256)` - Place a limit order
- `swap(address,address,uint256,uint256)` - Execute a swap
- `getQuote(address,address,uint256)` - Get a quote for a swap

## Differences from op-rbuilder

Unlike the external builder setup (`op-rbuilder` + `rollup-boost` + `op-geth`), this is a single binary that:

- **Simpler**: No coordination between multiple processes
- **No validation issues**: State and gas are all handled internally
- **Easier to debug**: All logic in one place
- **Perfect for MVP/demo**: Less infrastructure needed

## Development

The payload builder currently delegates to the standard OP payload builder for non-DEX transactions. To fully implement DEX transaction handling:

1. Modify `best_payload()` in `job.rs` to process transactions from the pool
2. Intercept DEX transactions and execute them via `handle_dex_transaction()`
3. Create proper receipts with logs for DEX operations
4. Ensure gas calculations match EVM execution

## Future Work

- Implement full transaction processing in `best_payload()`
- Add proper log emission for DEX events
- Optimize DEX state management
- Add metrics and monitoring
- Support for state persistence across node restarts
