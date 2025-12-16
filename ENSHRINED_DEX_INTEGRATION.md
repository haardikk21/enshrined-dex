# Enshrined DEX Integration with op-rbuilder

This document explains how the enshrined DEX is integrated with the op-rbuilder Flashblocks builder.

## Overview

The enshrined DEX is integrated as a **system predeploy contract** at a fixed address. Users interact with it like a normal smart contract, but the actual execution happens in the protocol layer using the in-memory `enshrined-dex` library.

## Architecture

```
User Transaction
    ↓
[To: 0x4200000000000000000000000000000000000042]
    ↓
Flashblocks Builder
    ↓
DEX Handler (intercepts transaction)
    ↓
enshrined-dex library (in-memory execution)
    ↓
Receipt + Logs emitted
```

## Components

### 1. Predeploy Contract (`contracts/src/EnshrinedDEX.sol`)

**Address**: `0x4200000000000000000000000000000000000042`

The Solidity interface that users interact with. This is just an interface - the actual implementation is in the protocol layer.

**Key Functions**:
- `createPair(address token0, address token1)` - Create a new trading pair (**RESTRICTED**: only whitelisted addresses)
- `placeLimitOrder(...)` - Place a limit order on the orderbook
- `cancelOrder(bytes32 orderId)` - Cancel an existing order
- `swap(address tokenIn, address tokenOut, uint256 amountIn, uint256 minAmountOut)` - Execute a swap
- `getQuote(address tokenIn, address tokenOut, uint256 amountIn)` - Get a quote for a swap (view function)

### 2. DEX Handler (`op-rbuilder/crates/op-rbuilder/src/dex/`)

The Rust implementation that handles DEX transactions in the protocol layer.

**Structure**:
```
dex/
├── mod.rs              # Module exports
├── predeploy.rs        # Predeploy address and function selectors
├── types.rs            # Type conversions and result encoding
└── handler.rs          # Main transaction handler logic
```

**How it works**:
1. Receives transaction targeting the predeploy address
2. Decodes the calldata using function selectors
3. Executes operation on the in-memory `PoolManager`
4. Returns results as proper EVM receipts with logs

### 3. Flashblocks Integration (`op-rbuilder/src/builders/flashblocks/dex_integration.rs`)

Integrates the DEX handler into the Flashblocks execution flow.

**Key Functions**:
- `is_dex_transaction(tx)` - Checks if a transaction targets the DEX
- `execute_dex_transaction(...)` - Executes a DEX transaction and creates proper receipts
- Gas estimation for DEX operations
- Event log creation for DEX operations

## How to Use

### For Users (Sending Transactions)

Users send normal Ethereum transactions to the predeploy address:

```javascript
// Using ethers.js or similar
const dexAddress = "0x4200000000000000000000000000000000000042";
const dex = new ethers.Contract(dexAddress, DEX_ABI, signer);

// Create a pair (ONLY whitelisted addresses can do this!)
// This would typically be called by governance or the sequencer
await dex.createPair(ETH_ADDRESS, USDC_ADDRESS);

// Place a limit order
await dex.placeLimitOrder(
    USDC_ADDRESS,  // tokenIn
    ETH_ADDRESS,   // tokenOut
    false,         // isBuy (false = sell order)
    parseUnits("1000", 6),  // amount (1000 USDC)
    2000,          // priceNum
    1,             // priceDenom (price = 2000/1)
);

// Execute a swap
await dex.swap(
    USDC_ADDRESS,
    ETH_ADDRESS,
    parseUnits("100", 6),   // amountIn
    parseEther("0.04")      // minAmountOut (slippage protection)
);

// Get a quote
const [amountOut, route] = await dex.getQuote(
    USDC_ADDRESS,
    ETH_ADDRESS,
    parseUnits("100", 6)
);
```

### For Validators

Validators need to include the same DEX handler logic to validate blocks:

1. **Install the enshrined-dex library** in your validator node
2. **Intercept transactions to the predeploy address** during block validation
3. **Execute using the DEX handler** instead of normal EVM execution
4. **Verify receipts and state transitions match**

The DEX state is deterministic - same inputs produce same outputs, so validators will reach consensus.

## Whitelist Management

The `DexHandler` includes a whitelist for addresses allowed to create new trading pairs. This restricts which tokens can be listed on the enshrined DEX.

**Initializing with whitelist**:
```rust
// Create handler with initial whitelist
let whitelist = vec![
    governance_address,
    sequencer_address,
];
let dex_handler = DexHandler::new_with_whitelist(whitelist);

// Or add addresses dynamically
let dex_handler = DexHandler::new();
dex_handler.add_to_whitelist(governance_address);
```

**Managing the whitelist**:
```rust
// Add an address
dex_handler.add_to_whitelist(new_admin_address);

// Remove an address
dex_handler.remove_from_whitelist(old_admin_address);

// Check if whitelisted
if dex_handler.is_whitelisted(caller) {
    // Can create pairs
}
```

## Integration Points in op-rbuilder

To complete the integration, you need to modify the transaction execution flow:

### In `payload.rs` (Flashblocks Builder)

Add DEX transaction interception in the transaction execution loop:

```rust
// In execute_transactions() or similar
use crate::dex::{DexHandler, dex_integration};

// Initialize DEX handler (once per block building session)
let dex_handler = DexHandler::new();

// In the transaction loop:
for tx in transactions {
    let sender = recover_signer(&tx)?;
    
    // Check if this is a DEX transaction
    if dex_integration::is_dex_transaction(&tx) {
        // Execute via DEX handler instead of EVM
        dex_integration::execute_dex_transaction(
            &dex_handler,
            &tx,
            sender,
            &mut info,
        )?;
        continue; // Skip normal EVM execution
    }
    
    // Normal transaction execution...
}
```

### In `payload_handler.rs` (Sync Handler)

Similarly, when syncing flashblocks from peers, intercept DEX transactions:

```rust
// In execute_flashblock()
let dex_handler = DexHandler::new();

for tx in payload.block().body().transactions {
    if dex_integration::is_dex_transaction(&tx) {
        dex_integration::execute_dex_transaction(
            &dex_handler,
            &tx,
            sender,
            &mut info,
        )?;
        continue;
    }
    // Normal execution...
}
```

## State Management

### DEX State Storage

Currently, the DEX state lives entirely in-memory in the `PoolManager`. For production, you have several options:

1. **Embed in block metadata** - Include serialized DEX state in block extra_data
2. **Separate state tree** - Create a separate trie for DEX state (like Ethereum's storage trie)
3. **Custom storage backend** - Use a database with the block hash as the key
4. **Hybrid approach** - Store state root hash in block, full state in auxiliary storage

### State Synchronization

Validators need to:
1. Start with the genesis DEX state (empty `PoolManager`)
2. Apply DEX transactions in order as they appear in blocks
3. Verify the resulting state matches the state commitment in the block

## Gas Estimation

Current gas estimates for DEX operations:

- `createPair`: 100,000 gas
- `placeLimitOrder`: 150,000 gas
- `cancelOrder`: 50,000 gas
- `swap`: 200,000 gas
- `getQuote`: 0 gas (view function)

These can be tuned based on actual computational cost.

## Events and Logs

DEX operations emit standard Ethereum events:

- `PairCreated(address indexed token0, address indexed token1, bytes32 indexed pairId)`
- `LimitOrderPlaced(bytes32 indexed orderId, address indexed trader, ...)`
- `OrderCancelled(bytes32 indexed orderId, address indexed trader)`
- `Swap(address indexed trader, address indexed tokenIn, address indexed tokenOut, ...)`

These events are included in transaction receipts and can be queried like normal contract events.

## Next Steps: Gas-in-Token Transactions

For the gas-in-token feature (where users pay gas fees in tokens other than ETH), you'll need to:

1. **Define a custom transaction type** (extending `OpTransactionSigned`)
2. **Add fields for gas token and pricing**
3. **Implement pre-execution swap** (gas token → ETH via enshrined DEX)
4. **Handle refunds in gas token**
5. **Ensure validator compatibility**

This is a more complex change that requires modifying the transaction pool, validation logic, and consensus rules. The custom transaction type approach will be documented separately.

## Testing

To test the DEX integration:

1. **Deploy the predeploy contract** at the correct address in your test environment
2. **Send test transactions** to the predeploy address
3. **Verify receipts and events** are emitted correctly
4. **Test multi-hop routing** with multiple pairs
5. **Test flashblock building** with DEX transactions interspersed

## Deployment Checklist

Before deploying to production:

- [ ] Decide on state storage strategy
- [ ] Implement state persistence and recovery
- [ ] Add comprehensive error handling
- [ ] Implement proper gas metering based on actual costs
- [ ] Add monitoring and metrics for DEX operations
- [ ] Test with high transaction volume
- [ ] Ensure validator nodes have the DEX handler
- [ ] Document migration path for existing chains
- [ ] Add emergency pause functionality
- [ ] Audit the DEX integration code

## File Reference

### Solidity Files
- `contracts/src/EnshrinedDEX.sol` - Predeploy interface contract

### Rust Files (op-rbuilder)
- `src/dex/mod.rs` - Module exports
- `src/dex/predeploy.rs` - Constants and selectors
- `src/dex/types.rs` - Type conversions
- `src/dex/handler.rs` - Main DEX handler
- `src/builders/flashblocks/dex_integration.rs` - Flashblocks integration

### Dependencies
- `crates/dex` - The enshrined-dex library (in-memory orderbook)
- Added to `op-rbuilder/Cargo.toml`

## Example Flow: Swap Transaction

1. **User sends transaction**:
   ```
   To: 0x4200000000000000000000000000000000000042
   Data: 0x128acb08... (swap function selector + params)
   Value: 0
   ```

2. **Flashblocks builder receives transaction**:
   - Recognizes destination address as DEX predeploy
   - Routes to `dex_integration::execute_dex_transaction()`

3. **DEX handler processes**:
   - Decodes calldata: `swap(USDC, ETH, 100e6, 0.04e18)`
   - Calls `pool_manager.execute_swap(...)`
   - Gets result: `amountOut = 0.048e18`

4. **Receipt created**:
   - Status: Success (1)
   - Gas used: 200,000
   - Logs: `Swap(trader, USDC, ETH, 100e6, 0.048e18, route)`

5. **User receives**:
   - Transaction receipt with success status
   - Can query logs to see swap details
   - Balance updated (handled separately via balance tracking)

## Notes

- The current implementation does **NOT** handle actual token transfers - that would need to be integrated with the EVM state for ERC20 balances
- For ETH (address(0)), you'd need to update the account balance in the EVM state
- Production deployment needs proper balance tracking integrated with the state DB
- Consider adding a "DEX balance" tracking system separate from ERC20s for better performance
