import { decodeEventLog, type Log } from 'viem'
import { COMBINED_ABI } from './abis'

export interface DecodedLog {
  eventName: string
  args: Record<string, unknown>
  success: true
}

export interface FailedDecode {
  success: false
  error: string
}

export type DecodeResult = DecodedLog | FailedDecode

/**
 * Attempts to decode a log using known ABIs
 */
export function decodeLog(log: Log): DecodeResult {
  try {
    // Try to decode the log with our combined ABI
    const decoded = decodeEventLog({
      abi: COMBINED_ABI,
      data: log.data,
      topics: log.topics as [`0x${string}`, ...`0x${string}`[]]
    })

    return {
      success: true,
      eventName: decoded.eventName,
      args: decoded.args as Record<string, unknown>
    }
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown decoding error'
    }
  }
}

/**
 * Formats a decoded argument value for display
 */
export function formatArg(name: string, value: unknown): string {
  // Handle different value types
  if (typeof value === 'bigint') {
    // For large numbers, show both decimal and hex
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
      return `${value.toString()} (0x${value.toString(16)})`
    }
    return value.toString()
  }

  if (typeof value === 'boolean') {
    return value.toString()
  }

  if (typeof value === 'string') {
    // Check if it looks like an address
    if (value.startsWith('0x') && value.length === 42) {
      return value
    }
    return value
  }

  if (Array.isArray(value)) {
    return `[${value.length} items]`
  }

  return String(value)
}

/**
 * Gets a human-readable description of the event
 */
export function getEventDescription(eventName: string, args: Record<string, unknown>): string {
  switch (eventName) {
    case 'PairCreated':
      return `Trading pair created between ${args.token0} and ${args.token1}`

    case 'LimitOrderPlaced':
      return `Limit order placed: ${args.isBuy ? 'Buy' : 'Sell'} ${args.amount} tokens`

    case 'OrderCancelled':
      return `Order ${args.orderId} cancelled by ${args.trader}`

    case 'Swap':
      return `Swap: ${args.amountIn} ${args.tokenIn} → ${args.amountOut} ${args.tokenOut}`

    case 'Transfer':
      return `Transfer: ${args.amount} tokens from ${args.from} to ${args.to}`

    case 'Approval':
      return `Approval: ${args.owner} approved ${args.spender} to spend ${args.amount} tokens`

    case 'LiquidityAdded':
      return `Liquidity added: ${args.amount0} token0 + ${args.amount1} token1`

    case 'LiquidityRemoved':
      return `Liquidity removed: ${args.amount0} token0 + ${args.amount1} token1`

    default:
      return eventName
  }
}
