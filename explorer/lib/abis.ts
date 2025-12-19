// ABI definitions for common contracts
export const ENSHRINED_DEX_ABI = [
  {
    "type": "event",
    "name": "PairCreated",
    "inputs": [
      { "name": "token0", "type": "address", "indexed": true },
      { "name": "token1", "type": "address", "indexed": true },
      { "name": "pairId", "type": "bytes32", "indexed": true }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "LimitOrderPlaced",
    "inputs": [
      { "name": "orderId", "type": "bytes32", "indexed": true },
      { "name": "trader", "type": "address", "indexed": true },
      { "name": "tokenIn", "type": "address", "indexed": true },
      { "name": "tokenOut", "type": "address", "indexed": false },
      { "name": "isBuy", "type": "bool", "indexed": false },
      { "name": "amount", "type": "uint256", "indexed": false },
      { "name": "priceNum", "type": "uint256", "indexed": false },
      { "name": "priceDenom", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "OrderCancelled",
    "inputs": [
      { "name": "orderId", "type": "bytes32", "indexed": true },
      { "name": "trader", "type": "address", "indexed": true }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "OrderFilled",
    "inputs": [
      { "name": "makerOrderId", "type": "bytes32", "indexed": true },
      { "name": "takerOrderId", "type": "bytes32", "indexed": true },
      { "name": "amount", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Swap",
    "inputs": [
      { "name": "trader", "type": "address", "indexed": true },
      { "name": "tokenIn", "type": "address", "indexed": true },
      { "name": "tokenOut", "type": "address", "indexed": true },
      { "name": "amountIn", "type": "uint256", "indexed": false },
      { "name": "amountOut", "type": "uint256", "indexed": false },
      { "name": "route", "type": "bytes32[]", "indexed": false }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "LiquidityAdded",
    "inputs": [
      { "name": "provider", "type": "address", "indexed": true },
      { "name": "token0", "type": "address", "indexed": true },
      { "name": "token1", "type": "address", "indexed": true },
      { "name": "amount0", "type": "uint256", "indexed": false },
      { "name": "amount1", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "LiquidityRemoved",
    "inputs": [
      { "name": "provider", "type": "address", "indexed": true },
      { "name": "token0", "type": "address", "indexed": true },
      { "name": "token1", "type": "address", "indexed": true },
      { "name": "amount0", "type": "uint256", "indexed": false },
      { "name": "amount1", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  }
] as const

export const ERC20_ABI = [
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      { "name": "from", "type": "address", "indexed": true },
      { "name": "to", "type": "address", "indexed": true },
      { "name": "amount", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Approval",
    "inputs": [
      { "name": "owner", "type": "address", "indexed": true },
      { "name": "spender", "type": "address", "indexed": true },
      { "name": "amount", "type": "uint256", "indexed": false }
    ],
    "anonymous": false
  }
] as const

// Combined ABI for all known contracts
export const COMBINED_ABI = [
  ...ENSHRINED_DEX_ABI,
  ...ERC20_ABI
] as const
