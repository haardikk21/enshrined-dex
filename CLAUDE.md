# Enshrined DEX - Project Summary

## Overview

This is an in-memory orderbook DEX library written in Rust. It provides efficient order matching, multi-pair management, and automatic trade routing without any blockchain calls - purely a data structure and algorithm library.

## Project Structure

```
enshrined-dex/
├── Cargo.toml              # Workspace root
├── CLAUDE.md               # This file
└── crates/
    └── dex/
        ├── Cargo.toml
        ├── src/
        │   ├── lib.rs          # Public API exports
        │   ├── types.rs        # Core types (TokenId, Amount, Price, U256, Address)
        │   ├── config.rs       # DexConfig (fees, routing hops, min order size)
        │   ├── order.rs        # Order, OrderId, OrderSide, OrderType, OrderStatus
        │   ├── pair.rs         # Pair, PairId, PairStats
        │   ├── orderbook.rs    # OrderBook with limit/market order matching
        │   ├── pool_manager.rs # PoolManager - manages all pairs, quotes, swaps
        │   └── router.rs       # Multi-hop routing (BFS pathfinding)
        └── tests/
            └── e2e.rs          # End-to-end integration tests
```

## Dependencies

- `alloy` (full features) - Ethereum types (Address, U256, keccak256, etc.)

## Key Components

### Types (`types.rs`)
- `TokenId` = `Address` (token contract address, ETH = Address::ZERO)
- `Amount` = `U256` (token amounts in smallest units)
- `Price` - Rational number (numerator/denominator) for precise pricing

### Config (`config.rs`)
- `fee_bps` - Fee in basis points (default: 30 = 0.30%)
- `max_routing_hops` - Max hops for multi-hop routing (default: 3)
- `min_order_size` - Minimum order size (default: 1)
- `allow_self_trade` - Whether same address can trade with itself (default: false)

### OrderBook (`orderbook.rs`)
- Uses `BTreeMap` for price levels (chosen over heaps for O(log n) cancellation and iteration)
- Buy orders sorted descending (highest first)
- Sell orders sorted ascending (lowest first)
- Price-time priority matching
- Supports limit and market orders

### PoolManager (`pool_manager.rs`)
- Manages multiple trading pairs
- Creates pairs, places orders, cancels orders
- Gets quotes (direct or routed)
- Executes swaps with slippage protection

### Router (`router.rs`)
- BFS-based pathfinding between tokens
- Finds all routes up to max_hops
- Evaluates routes to find best output
- Used automatically when no direct pair exists

## Usage Example

```rust
use dex::{PoolManager, OrderSide, Price, Address, U256};

let mut pm = PoolManager::new();

// Create a pair
let eth = Address::ZERO;
let usdc = Address::repeat_byte(0x01);
pm.create_pair(eth, usdc)?;

// Place a limit order (sell 1 ETH at $2000)
pm.place_limit_order(
    eth, usdc, trader_address,
    OrderSide::Sell,
    Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
    U256::from(10u64.pow(18)),
)?;

// Get a quote
let quote = pm.get_quote(usdc, eth, usdc_amount)?;

// Execute swap with slippage protection
let result = pm.execute_swap(trader, usdc, eth, amount_in, min_amount_out)?;
```

## Tests

- **Unit tests**: 24 tests in individual modules
- **E2E tests**: 25 tests in `tests/e2e.rs`

Run all tests:
```bash
cargo test
```

## Design Decisions

1. **BTreeMap over Heaps**: Chose BTreeMap for orderbook because:
   - O(log n) cancellation (heaps are O(n))
   - Can iterate through price levels in order
   - Supports range queries for depth display

2. **Rational Prices**: Using numerator/denominator instead of floats for precision

3. **Multi-hop Routing**: BFS finds all paths, then evaluates each to find best output

4. **Fee on Output**: Fees are deducted from the output amount after matching

## Future Considerations

- The library is purely in-memory with no persistence
- No blockchain integration - designed to be used by a higher-level system
- Could add WebSocket/API layer for real-time orderbook updates
- Could add persistence layer for order recovery
