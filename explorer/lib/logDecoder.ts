import { decodeEventLog, type Log } from 'viem'
import { COMBINED_ABI } from './abis'
import { getNameTag, formatTokenAmount } from './nametags'

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
 * Format an address for display - use nametag if available, otherwise truncate
 */
function formatAddressForDescription(address: unknown): string {
  if (typeof address !== 'string') return String(address)
  const nameTag = getNameTag(address)
  if (nameTag) return nameTag
  return `${address.slice(0, 6)}...${address.slice(-4)}`
}

/**
 * Gets a human-readable description of the event
 */
export function getEventDescription(eventName: string, args: Record<string, unknown>, logAddress?: string): string {
  try {
    switch (eventName) {
      case 'PairCreated':
        return `Trading pair created between ${formatAddressForDescription(args.token0)} and ${formatAddressForDescription(args.token1)}`

      case 'LimitOrderPlaced': {
        const tokenIn = args.tokenIn
        const amount = args.amount
        if (typeof tokenIn === 'string' && typeof amount === 'bigint') {
          const formattedAmount = formatTokenAmount(amount, tokenIn)
          return `Limit order placed: ${args.isBuy ? 'Buy' : 'Sell'} ${formattedAmount} ${formatAddressForDescription(tokenIn)}`
        }
        return `Limit order placed: ${args.isBuy ? 'Buy' : 'Sell'}`
      }

      case 'OrderCancelled':
        return `Order cancelled by ${formatAddressForDescription(args.trader)}`

      case 'Swap': {
        const tokenIn = args.tokenIn
        const tokenOut = args.tokenOut
        const amountIn = args.amountIn
        const amountOut = args.amountOut
        if (typeof tokenIn === 'string' && typeof tokenOut === 'string' &&
            typeof amountIn === 'bigint' && typeof amountOut === 'bigint') {
          const formattedIn = formatTokenAmount(amountIn, tokenIn)
          const formattedOut = formatTokenAmount(amountOut, tokenOut)
          return `Swap: ${formattedIn} ${formatAddressForDescription(tokenIn)} → ${formattedOut} ${formatAddressForDescription(tokenOut)}`
        }
        return 'Swap executed'
      }

      case 'Transfer': {
        const amount = args.value ?? args.amount
        if (typeof amount === 'bigint' && logAddress) {
          const formattedAmount = formatTokenAmount(amount, logAddress)
          const tokenName = formatAddressForDescription(logAddress)
          return `Transfer: ${formattedAmount} ${tokenName} from ${formatAddressForDescription(args.from)} to ${formatAddressForDescription(args.to)}`
        } else if (typeof amount === 'bigint') {
          return `Transfer: ${amount.toString()} from ${formatAddressForDescription(args.from)} to ${formatAddressForDescription(args.to)}`
        }
        return `Transfer from ${formatAddressForDescription(args.from)} to ${formatAddressForDescription(args.to)}`
      }

      case 'Approval': {
        const amount = args.value ?? args.amount
        if (typeof amount === 'bigint' && logAddress) {
          const formattedAmount = formatTokenAmount(amount, logAddress)
          const tokenName = formatAddressForDescription(logAddress)
          return `Approval: ${formatAddressForDescription(args.owner)} approved ${formatAddressForDescription(args.spender)} for ${formattedAmount} ${tokenName}`
        } else if (typeof amount === 'bigint') {
          return `Approval: ${formatAddressForDescription(args.owner)} approved ${formatAddressForDescription(args.spender)} for ${amount.toString()}`
        }
        return `Approval: ${formatAddressForDescription(args.owner)} approved ${formatAddressForDescription(args.spender)}`
      }

      case 'LiquidityAdded': {
        const amount0 = args.amount0
        const amount1 = args.amount1
        if (typeof amount0 === 'bigint' && typeof amount1 === 'bigint') {
          return `Liquidity added: ${amount0.toString()} + ${amount1.toString()}`
        }
        return 'Liquidity added'
      }

      case 'LiquidityRemoved': {
        const amount0 = args.amount0
        const amount1 = args.amount1
        if (typeof amount0 === 'bigint' && typeof amount1 === 'bigint') {
          return `Liquidity removed: ${amount0.toString()} + ${amount1.toString()}`
        }
        return 'Liquidity removed'
      }

      default:
        return eventName
    }
  } catch (error) {
    // Fallback in case of any unexpected errors
    return eventName
  }
}
