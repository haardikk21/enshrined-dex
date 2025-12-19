import { decodeEventLog, type Log } from 'viem'
import { COMBINED_ABI } from './abis'
import { formatTokenAmount, getTokenDecimals } from './nametags'

export interface Order {
  orderId: string
  trader: string
  price: number
  amount: number
  total: number
  tokenIn: string
  tokenOut: string
  isBuy: boolean
}

export interface Orderbook {
  buyOrders: Order[]
  sellOrders: Order[]
}

/**
 * Parse LimitOrderPlaced and OrderFilled events to reconstruct current orderbook state
 */
export function parseOrderbookFromLogs(
  logs: Log[],
  token0: string,
  token1: string
): Orderbook {
  // Track all orders by orderId
  const ordersById = new Map<string, Order>()

  // Track which orders have been filled (fully or partially)
  const filledOrderIds = new Set<string>()

  // Normalize addresses for comparison
  const normalizedToken0 = token0.toLowerCase()
  const normalizedToken1 = token1.toLowerCase()

  // First pass: collect all orders and identify filled orders
  for (const log of logs) {
    try {
      const decoded = decodeEventLog({
        abi: COMBINED_ABI,
        data: log.data,
        topics: log.topics as [`0x${string}`, ...`0x${string}`[]]
      })

      if (decoded.eventName === 'OrderFilled') {
        // Mark both maker and taker orders as filled
        const args = decoded.args as any
        if (args.makerOrderId) {
          filledOrderIds.add(args.makerOrderId as string)
        }
        if (args.takerOrderId) {
          filledOrderIds.add(args.takerOrderId as string)
        }
        continue
      }

      if (decoded.eventName === 'LimitOrderPlaced') {
        const args = decoded.args as any

        // Check if this order belongs to the pair we're interested in
        const orderTokenIn = (args.tokenIn as string).toLowerCase()
        const orderTokenOut = (args.tokenOut as string).toLowerCase()

        const isPairOrder =
          (orderTokenIn === normalizedToken0 && orderTokenOut === normalizedToken1) ||
          (orderTokenIn === normalizedToken1 && orderTokenOut === normalizedToken0)

        if (!isPairOrder) continue

        // Calculate price as a number, accounting for token decimals
        const priceNum = args.priceNum as bigint
        const priceDenom = args.priceDenom as bigint

        // Normalize price to always be in terms of token0/token1 (not order direction)
        // For the pair, we want price to always mean: "how much token1 per token0"
        // token0 = normalizedToken0, token1 = normalizedToken1

        // The price in the order is: priceNum/priceDenom where:
        // - For buy orders (buying base with quote): priceNum is in quote token decimals
        // - For sell orders (selling base for quote): priceNum is in quote token decimals
        // - priceDenom is always 10^18 (base token decimals)

        // Since we're looking at ETH/USDC or ETH/DAI pairs where ETH is token0:
        // We want price to mean "USDC per ETH" or "DAI per ETH"

        const isBuy = args.isBuy as boolean
        const amount = args.amount as bigint
        let price: number

        if (isBuy) {
          // Buy order: tokenIn = quote, tokenOut = base
          // priceNum is in quote decimals, priceDenom is in base decimals
          const quoteDecimals = getTokenDecimals(orderTokenIn)
          const baseDecimals = getTokenDecimals(orderTokenOut)
          price = (Number(priceNum) / Math.pow(10, quoteDecimals)) / (Number(priceDenom) / Math.pow(10, baseDecimals))
        } else {
          // Sell order: tokenIn = base, tokenOut = quote
          // priceNum is in quote decimals, priceDenom is in base decimals
          const quoteDecimals = getTokenDecimals(orderTokenOut)
          const baseDecimals = getTokenDecimals(orderTokenIn)
          price = (Number(priceNum) / Math.pow(10, quoteDecimals)) / (Number(priceDenom) / Math.pow(10, baseDecimals))
        }

        // Calculate the output amount
        let outputAmount: bigint
        try {
          outputAmount = (amount * priceDenom) / priceNum
        } catch {
          continue // Skip if calculation fails
        }

        // Format amounts based on token
        const formattedInputAmount = Number(formatTokenAmount(amount, orderTokenIn))
        const formattedOutputAmount = Number(formatTokenAmount(outputAmount, orderTokenOut))

        const order: Order = {
          orderId: args.orderId as string,
          trader: args.trader as string,
          price,
          amount: isBuy ? formattedOutputAmount : formattedInputAmount,
          total: 0, // Will be calculated later
          tokenIn: orderTokenIn,
          tokenOut: orderTokenOut,
          isBuy,
        }

        ordersById.set(order.orderId, order)
      }
    } catch (error) {
      // Skip logs that can't be decoded
      continue
    }
  }

  // Second pass: filter out filled orders
  const activeOrders: Order[] = []

  for (const [orderId, order] of ordersById.entries()) {
    if (!filledOrderIds.has(orderId)) {
      // This order has not been filled
      activeOrders.push(order)
    }
  }

  // Separate into buy and sell orders
  const buyOrders = activeOrders.filter(o => o.isBuy)
  const sellOrders = activeOrders.filter(o => !o.isBuy)

  // Sort orders: buy orders by price descending, sell orders by price ascending
  buyOrders.sort((a, b) => a.price - b.price)
  sellOrders.sort((a, b) => a.price - b.price)

  // Calculate cumulative totals
  let buyTotal = 0
  for (const order of buyOrders) {
    buyTotal += order.amount
    order.total = buyTotal
  }

  let sellTotal = 0
  for (const order of sellOrders) {
    sellTotal += order.amount
    order.total = sellTotal
  }

  return { buyOrders, sellOrders }
}
