<h1 align="center">Enshrined DEX</h1>

<p align="center">
  <img width="900" src="./header.jpg" alt="Enshrined DEX" />
</p>

---

A custom Optimism L2 node with a protocol-level orderbook DEX. Instead of running DEX logic in smart contracts, trades are executed directly in the node's state transition function for maximum efficiency. Built during the Base H2 2025 internal hackathon.

## What is an Enshrined DEX?

Traditional DEXs run as smart contracts on top of the EVM, paying gas for every operation. An **enshrined DEX** moves the exchange logic into the protocol itself:

- Transactions to the DEX predeploy (`0x4200000000000000000000000000000000000042`) are intercepted by the node
- Order matching happens in native Rust code, not EVM bytecode
- State changes (token transfers, order updates) are applied directly
- Events are emitted as if the contract executed normally

This approach provides near-zero execution cost for trades while maintaining full EVM compatibility for everything else.

## Why?

Onchain DEXs like Aerodrome and Uniswap are great, but they have fundamental limits. Constant product market makers (CPMMs) work well for many use cases, but high-performance orderbooks are superior for serious trading: better price discovery, tighter spreads, and no slippage on limit orders.

The problem is that orderbooks don't work well onchain. Filling a market order requires looping through an unknown number of resting orders, and this iteration can easily exceed block gas limits. Even when it doesn't, the gas costs make it impractical. So we settle for AMMs.

**The enshrined DEX solves this by moving the orderbook out of the EVM entirely.** It's a custom Reth node running a modified OP Stack chain, with a DEX predeploy that executes natively as Rust code directly within the sequencer. No gas limits on matching, no EVM overhead. The project also includes a minimal block explorer and testing scripts for demonstration.

### Beyond Efficient Trading

Enshrining a DEX directly in the block builder opens up more than just efficient trading. With the sequencer having native access to an orderbook, we can implement true native account abstraction.

ERC-4337 style account abstraction exists mainly for paymaster functionality, but it's clunky. With an enshrined DEX, users can **pay gas in any token**. The sequencer automatically swaps the user's token for ETH during block building, completely transparently.

### Future Possibilities

With a base-layer orderbook directly in the sequencer, the architecture naturally extends to:

- **Native enshrined derivatives** - perpetuals, options, and synthetics with the same gas efficiency
- **Interchangeable stablecoins** - seamless swaps between USDC, custom-branded stablecoins (like Coinbase's merchant stablecoins), and other stable assets at the protocol level

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      User Transaction                        │
│                  (to DEX predeploy address)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Reth Node (reth-node)                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Custom Payload Builder                 │    │
│  │  • Intercepts DEX transactions                      │    │
│  │  • Routes to DexHandler                             │    │
│  │  • Generates receipts and logs                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                              │                              │
│                              ▼                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   DEX Handler                       │    │
│  │  • Decodes calldata (createPair, swap, etc.)        │    │
│  │  • Validates ETH value and parameters               │    │
│  │  • Calls into PoolManager                           │    │
│  └─────────────────────────────────────────────────────┘    │
│                              │                              │
│                              ▼                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              PoolManager (dex crate)                │    │
│  │  • In-memory orderbook (BTreeMap)                   │    │
│  │  • Price-time priority matching                     │    │
│  │  • Multi-hop routing (BFS pathfinding)              │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## DEX Features

### Supported Operations

| Function | Description |
|----------|-------------|
| `createPair(token0, token1)` | Create a new trading pair |
| `placeLimitOrder(tokenIn, tokenOut, isBuy, amount, priceNum, priceDenom)` | Place a limit order with rational price |
| `cancelOrder(orderId)` | Cancel an existing order |
| `swap(tokenIn, tokenOut, amountIn, minAmountOut)` | Market swap with slippage protection |
| `getQuote(tokenIn, tokenOut, amountIn)` | Get expected output for a swap |

## Quick Start

### Prerequisites

- Rust (latest stable)
- [Just](https://github.com/casey/just) (task runner)
- [Bun](https://bun.sh) (for scripts/explorer)
- [Foundry](https://getfoundry.sh) (for contract compilation)

### Build

```bash
# Build the node
cargo build -p reth-node --release

# Build contracts (for ABI generation)
cd contracts && forge build
```

### Run the Node

```bash
# Start in dev mode with 1-second block time
just run

# Or manually:
RUST_LOG=dex=debug,payload_builder=debug cargo run -p reth-node -- node \
    --datadir ./dex-reth \
    --dev \
    --dev.block-time 1s
```

### Interact with the DEX

```bash
# Run the test script
just run-script

# Start the explorer UI
just run-explorer
```

## Example Usage

```typescript
import { createWalletClient, http, parseEther } from 'viem';

const DEX_ADDRESS = '0x4200000000000000000000000000000000000042';

// Create a trading pair (ETH/USDC)
await walletClient.sendTransaction({
  to: DEX_ADDRESS,
  data: encodeFunctionData({
    abi: EnshrinedDEXAbi,
    functionName: 'createPair',
    args: ['0x0000000000000000000000000000000000000000', USDC_ADDRESS]
  })
});

// Place a limit order: Sell 1 ETH at 2000 USDC
await walletClient.sendTransaction({
  to: DEX_ADDRESS,
  value: parseEther('1'),
  data: encodeFunctionData({
    abi: EnshrinedDEXAbi,
    functionName: 'placeLimitOrder',
    args: [
      '0x0000000000000000000000000000000000000000', // ETH (tokenIn)
      USDC_ADDRESS,                                  // USDC (tokenOut)
      false,                                         // isBuy = false (selling ETH)
      parseEther('1'),                              // amount
      2000n * 10n ** 6n,                            // priceNum (2000 USDC)
      10n ** 18n                                     // priceDenom (per 1 ETH)
    ]
  })
});

// Execute a swap: Buy ETH with 1000 USDC
await walletClient.sendTransaction({
  to: DEX_ADDRESS,
  data: encodeFunctionData({
    abi: EnshrinedDEXAbi,
    functionName: 'swap',
    args: [
      USDC_ADDRESS,                                  // tokenIn
      '0x0000000000000000000000000000000000000000', // tokenOut (ETH)
      1000n * 10n ** 6n,                            // amountIn
      parseEther('0.4')                             // minAmountOut (slippage protection)
    ]
  })
});
```

## Events

The DEX emits standard Ethereum events for all operations:

```solidity
event PairCreated(address indexed token0, address indexed token1, bytes32 indexed pairId);
event LimitOrderPlaced(bytes32 indexed orderId, address indexed trader, address indexed tokenIn, ...);
event OrderCancelled(bytes32 indexed orderId, address indexed trader);
event OrderFilled(bytes32 indexed makerOrderId, bytes32 indexed takerOrderId, uint256 amount);
event Swap(address indexed trader, address indexed tokenIn, address indexed tokenOut, ...);
```

## Testing

```bash
# Run all Rust tests
cargo test

# Run DEX library tests only
cargo test -p dex
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
