import Link from 'next/link'
import { notFound } from 'next/navigation'
import { getTransaction, getTransactionReceipt, getBlock, formatValue, formatGas } from '@/lib/rpc'
import { LogDisplay } from '@/components/LogDisplay'

export const dynamic = 'force-dynamic'

interface PageProps {
  params: Promise<{
    hash: string
  }>
}

export default async function TransactionPage({ params }: PageProps) {
  const { hash } = await params
  const [tx, receipt] = await Promise.all([
    getTransaction(hash),
    getTransactionReceipt(hash),
  ])

  if (!tx) {
    notFound()
  }

  // Get block details for timestamp
  const block = tx.blockHash ? await getBlock(tx.blockHash) : null

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
          <h1 className="text-4xl font-bold text-white tracking-tight">Transaction Details</h1>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 lg:px-8 py-12">
        {/* Transaction Overview */}
        <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm mb-6">
          <div className="px-6 py-5 border-b border-white/10">
            <h2 className="text-xl font-semibold text-white">Overview</h2>
          </div>
          <div className="px-6 py-6">
            <dl className="grid grid-cols-1 gap-6">
              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Transaction Hash</dt>
                <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                  {tx.hash}
                </dd>
              </div>

              {receipt && (
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Status</dt>
                  <dd className="mt-1">
                    {receipt.status === 'success' ? (
                      <span className="inline-flex items-center px-3 py-1 rounded-lg text-sm font-medium bg-[#0052ff]/20 text-[#0052ff] border border-[#0052ff]/30">
                        Success
                      </span>
                    ) : (
                      <span className="inline-flex items-center px-3 py-1 rounded-lg text-sm font-medium bg-red-500/20 text-red-400 border border-red-500/30">
                        Failed
                      </span>
                    )}
                  </dd>
                </div>
              )}

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Block Number</dt>
                  <dd className="text-base text-white font-mono">
                    {tx.blockNumber ? (
                      <Link
                        href={`/blocks/${tx.blockHash}`}
                        className="text-[#0052ff] hover:text-[#0052ff]/80 transition-colors"
                      >
                        {tx.blockNumber.toString()}
                      </Link>
                    ) : (
                      'Pending'
                    )}
                  </dd>
                </div>

                {block && (
                  <div>
                    <dt className="text-sm font-medium text-white/50 mb-2">Timestamp</dt>
                    <dd className="text-base text-white">
                      {new Date(Number(block.timestamp) * 1000).toLocaleString()}
                    </dd>
                  </div>
                )}
              </div>

              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">From</dt>
                <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                  {tx.from}
                </dd>
              </div>

              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">To</dt>
                <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                  {tx.to || (
                    <span className="text-white/50 italic">
                      Contract Creation
                    </span>
                  )}
                </dd>
              </div>

              {receipt?.contractAddress && (
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Contract Address</dt>
                  <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                    {receipt.contractAddress}
                  </dd>
                </div>
              )}

              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Value</dt>
                <dd className="text-base text-white font-mono">
                  {formatValue(tx.value)}
                </dd>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Gas Price</dt>
                  <dd className="text-base text-white font-mono">
                    {tx.gasPrice ? `${(Number(tx.gasPrice) / 1e9).toFixed(2)} Gwei` : 'N/A'}
                  </dd>
                </div>

                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Gas Limit</dt>
                  <dd className="text-base text-white font-mono">
                    {formatGas(tx.gas)}
                  </dd>
                </div>
              </div>

              {receipt && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                  <div>
                    <dt className="text-sm font-medium text-white/50 mb-2">Gas Used</dt>
                    <dd className="text-base text-white font-mono">
                      {formatGas(receipt.gasUsed)}
                    </dd>
                  </div>

                  <div>
                    <dt className="text-sm font-medium text-white/50 mb-2">Transaction Fee</dt>
                    <dd className="text-base text-white font-mono">
                      {tx.gasPrice
                        ? `${(Number(receipt.gasUsed * tx.gasPrice) / 1e18).toFixed(8)} ETH`
                        : 'N/A'
                      }
                    </dd>
                  </div>
                </div>
              )}

              <div>
                <dt className="text-sm font-medium text-white/50 mb-2">Nonce</dt>
                <dd className="text-base text-white font-mono">
                  {tx.nonce}
                </dd>
              </div>

              {tx.input && tx.input !== '0x' && (
                <div>
                  <dt className="text-sm font-medium text-white/50 mb-2">Input Data</dt>
                  <dd className="text-sm text-white/90 font-mono break-all bg-white/5 p-3 rounded-lg">
                    {tx.input}
                  </dd>
                </div>
              )}
            </dl>
          </div>
        </div>

        {/* Logs */}
        {receipt && receipt.logs && receipt.logs.length > 0 && (
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">
                Logs ({receipt.logs.length})
              </h2>
            </div>
            <div className="px-6 py-6">
              <div className="space-y-4">
                {receipt.logs.map((log, index) => (
                  <LogDisplay key={index} log={log} index={index} />
                ))}
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
