import Link from 'next/link'
import { notFound } from 'next/navigation'
import { getBlock, getTransaction, truncateHash, formatTimestamp, formatGas } from '@/lib/rpc'

export const dynamic = 'force-dynamic'

interface PageProps {
  params: Promise<{
    hash: string
  }>
}

export default async function BlockPage({ params }: PageProps) {
  const { hash } = await params
  const block = await getBlock(hash)

  if (!block) {
    notFound()
  }

  // Fetch all transactions in the block
  const transactions = await Promise.all(
    (block.transactions || [])
      .filter((tx): tx is string => typeof tx === 'string')
      .map(txHash => getTransaction(txHash))
  )

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
          <h1 className="text-4xl font-bold text-white tracking-tight">Block Details</h1>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 lg:px-8 py-12">
        {/* Block Overview */}
        <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm mb-6">
          <div className="px-6 py-5 border-b border-white/10">
            <h2 className="text-xl font-semibold text-white">Overview</h2>
          </div>
          <div className="px-6 py-6">
            <dl className="grid grid-cols-1 gap-6 sm:grid-cols-2">
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Block Number</dt>
                <dd className="text-base text-white font-mono">
                  {block.number?.toString()}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Timestamp</dt>
                <dd className="text-base text-white">
                  {formatTimestamp(block.timestamp)}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-sm font-medium text-white/50 mb-2">Block Hash</dt>
                <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                  {block.hash}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-sm font-medium text-white/50 mb-2">Parent Hash</dt>
                <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                  {block.parentHash}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Gas Used</dt>
                <dd className="text-base text-white font-mono">
                  {formatGas(block.gasUsed)}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Gas Limit</dt>
                <dd className="text-base text-white font-mono">
                  {formatGas(block.gasLimit)}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Transactions</dt>
                <dd className="text-base text-white">
                  {block.transactions?.length || 0}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Size</dt>
                <dd className="text-base text-white">
                  {block.size?.toString()} bytes
                </dd>
              </div>
              {block.miner && (
                <div className="sm:col-span-2">
                  <dt className="text-sm font-medium text-white/50 mb-2">Miner</dt>
                  <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                    {block.miner}
                  </dd>
                </div>
              )}
              {block.baseFeePerGas && (
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Base Fee Per Gas</dt>
                  <dd className="text-base text-white font-mono">
                    {block.baseFeePerGas.toString()} wei
                  </dd>
                </div>
              )}
            </dl>
          </div>
        </div>

        {/* Transactions */}
        {transactions.length > 0 && (
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">
                Transactions ({transactions.length})
              </h2>
            </div>
            <div className="divide-y divide-white/5">
              {transactions.map((tx) => {
                if (!tx) return null
                return (
                  <Link
                    key={tx.hash}
                    href={`/tx/${tx.hash}`}
                    className="block px-6 py-4 hover:bg-white/5 transition-all duration-200"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium text-white font-mono mb-1">
                          {truncateHash(tx.hash)}
                        </div>
                        <div className="text-xs text-white/50 font-mono">
                          {truncateHash(tx.from)}
                          {tx.to && (
                            <>
                              {' → '}
                              {truncateHash(tx.to)}
                            </>
                          )}
                        </div>
                      </div>
                      <div className="ml-4 flex-shrink-0 text-right">
                        <div className="text-sm text-white/90 font-mono">
                          {tx.value ? `${(Number(tx.value) / 1e18).toFixed(4)} ETH` : '0 ETH'}
                        </div>
                      </div>
                    </div>
                  </Link>
                )
              })}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
