import Link from 'next/link'
import { notFound } from 'next/navigation'
import { getNameTag, formatTokenAmount } from '@/lib/nametags'
import { OrderbookChart } from '@/components/OrderbookChart'
import { getDexLogs } from '@/lib/rpc'
import { parseOrderbookFromLogs } from '@/lib/orderbookParser'
import { decodeEventLog, type Log } from 'viem'
import { COMBINED_ABI } from '@/lib/abis'

export const dynamic = 'force-dynamic'

const DEX_ADDRESS = '0x4200000000000000000000000000000000000042'

interface PageProps {
  params: Promise<{
    token0: string
    token1: string
  }>
}

export default async function PairPage({ params }: PageProps) {
  const { token0, token1 } = await params

  const token0Name = getNameTag(token0) || token0
  const token1Name = getNameTag(token1) || token1

  // Fetch all DEX logs and parse orderbook
  const logs = await getDexLogs(DEX_ADDRESS)
  const orderbook = parseOrderbookFromLogs(logs, token0, token1)

  // Filter swap transactions for this pair
  const swapLogs = logs.filter((log) => {
    try {
      const decoded = decodeEventLog({
        abi: COMBINED_ABI,
        data: log.data,
        topics: log.topics as [`0x${string}`, ...`0x${string}`[]],
      })
      if (decoded.eventName !== 'Swap') return false

      const args = decoded.args as any
      const tokenIn = (args.tokenIn as string).toLowerCase()
      const tokenOut = (args.tokenOut as string).toLowerCase()
      const t0 = token0.toLowerCase()
      const t1 = token1.toLowerCase()

      // Check if this swap involves our pair
      return (tokenIn === t0 && tokenOut === t1) || (tokenIn === t1 && tokenOut === t0)
    } catch {
      return false
    }
  }).reverse() // Most recent first

  return (
    <div className="min-h-screen bg-black">
      <header className="border-b border-white/10">
        <div className="max-w-7xl mx-auto px-6 lg:px-8 py-8">
          <Link href="/" className="text-[#0052ff] hover:text-[#0052ff]/80 text-sm mb-3 inline-flex items-center gap-2 font-mono transition-colors">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
            Back
          </Link>
          <h1 className="text-4xl font-bold text-white tracking-tight">
            {token0Name}/{token1Name}
          </h1>
          <p className="text-sm text-white/60 mt-2">Trading Pair Orderbook</p>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 lg:px-8 py-12">
        {/* Pair Overview */}
        <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm mb-6">
          <div className="px-6 py-5 border-b border-white/10">
            <h2 className="text-xl font-semibold text-white">Pair Details</h2>
          </div>
          <div className="px-6 py-6">
            <dl className="grid grid-cols-1 sm:grid-cols-3 gap-6">
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Base Token</dt>
                <dd className="text-base text-white font-mono">{token0Name}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Quote Token</dt>
                <dd className="text-base text-white font-mono">{token1Name}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Open Orders</dt>
                <dd className="text-base text-white font-mono">
                  {orderbook.buyOrders.length + orderbook.sellOrders.length}
                </dd>
              </div>
            </dl>
          </div>
        </div>

        {/* Orderbook Visualization */}
        <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
          <div className="px-6 py-5 border-b border-white/10">
            <h2 className="text-xl font-semibold text-white">Orderbook Depth</h2>
            <p className="text-sm text-white/50 mt-1">
              Visualizing {orderbook.buyOrders.length} buy orders and {orderbook.sellOrders.length} sell orders
            </p>
          </div>
          <div className="p-6">
            <OrderbookChart
              buyOrders={orderbook.buyOrders}
              sellOrders={orderbook.sellOrders}
              token0Name={token0Name}
              token1Name={token1Name}
            />
          </div>
        </div>

        {/* Orders and Transactions Grid */}
        <div className="mt-6 grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Buy Orders */}
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">Buy Orders</h2>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-white/5">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium text-white/50 uppercase tracking-wider">
                      Price
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-white/50 uppercase tracking-wider">
                      Amount
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {orderbook.buyOrders.slice(0, 20).map((order, i) => (
                    <tr key={i} className="hover:bg-white/5 transition-colors">
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-green-400 font-mono">
                        {order.price.toFixed(2)}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-white/90 font-mono">
                        {order.amount.toFixed(4)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {orderbook.buyOrders.length > 20 && (
              <div className="px-4 py-3 text-center text-xs text-white/50">
                +{orderbook.buyOrders.length - 20} more
              </div>
            )}
          </div>

          {/* Sell Orders */}
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">Sell Orders</h2>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-white/5">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium text-white/50 uppercase tracking-wider">
                      Price
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-white/50 uppercase tracking-wider">
                      Amount
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {orderbook.sellOrders.slice(0, 20).map((order, i) => (
                    <tr key={i} className="hover:bg-white/5 transition-colors">
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-red-400 font-mono">
                        {order.price.toFixed(2)}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-white/90 font-mono">
                        {order.amount.toFixed(4)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {orderbook.sellOrders.length > 20 && (
              <div className="px-4 py-3 text-center text-xs text-white/50">
                +{orderbook.sellOrders.length - 20} more
              </div>
            )}
          </div>

          {/* Transactions */}
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">Transactions</h2>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-white/5">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium text-white/50 uppercase tracking-wider">
                      Swap
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {swapLogs.length === 0 ? (
                    <tr>
                      <td className="px-4 py-8 text-center text-sm text-white/50">
                        No swaps yet
                      </td>
                    </tr>
                  ) : (
                    swapLogs.slice(0, 20).map((log) => {
                      const decoded = decodeEventLog({
                        abi: COMBINED_ABI,
                        data: log.data,
                        topics: log.topics as [`0x${string}`, ...`0x${string}`[]],
                      })
                      const args = decoded.args as any
                      const amountIn = args.amountIn as bigint
                      const amountOut = args.amountOut as bigint
                      const tokenIn = (args.tokenIn as string).toLowerCase()
                      const tokenOut = (args.tokenOut as string).toLowerCase()

                      // Format amounts with token names
                      const tokenInName = tokenIn === token0.toLowerCase() ? token0Name : token1Name
                      const tokenOutName = tokenOut === token0.toLowerCase() ? token0Name : token1Name
                      const formattedAmountIn = formatTokenAmount(amountIn, tokenIn)
                      const formattedAmountOut = formatTokenAmount(amountOut, tokenOut)

                      // Determine color based on direction
                      const isBuy = tokenIn === token1.toLowerCase()

                      // Create unique key from transaction hash and log index
                      const uniqueKey = `${log.transactionHash}-${log.logIndex}`

                      return (
                        <tr key={uniqueKey} className="hover:bg-white/5 transition-colors cursor-pointer">
                          <td className="px-4 py-3 text-xs text-white/90 font-mono">
                            <Link href={`/tx/${log.transactionHash}`} className="block">
                              <span className={isBuy ? 'text-green-400' : 'text-red-400'}>
                                {formattedAmountIn} {tokenInName}
                              </span>
                              <span className="text-white/50 mx-1">→</span>
                              <span className="text-white/90">
                                {formattedAmountOut} {tokenOutName}
                              </span>
                            </Link>
                          </td>
                        </tr>
                      )
                    })
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
