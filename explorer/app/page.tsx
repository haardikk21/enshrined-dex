import Link from "next/link";
import {
  getRecentBlocks,
  getRecentTransactions,
  truncateHash,
  formatTimestamp,
  formatValue,
} from "@/lib/rpc";
import { getNameTag } from "@/lib/nametags";

export const dynamic = "force-dynamic";

// Mock DEX pairs data - in production this would come from DEX state
const DEX_PAIRS = [
  {
    token0: "0x0000000000000000000000000000000000000000", // ETH
    token1: "0x5fbdb2315678afecb367f032d93f642f64180aa3", // USDC
    volume24h: "2 ETH",
    orders: 100,
  },
  {
    token0: "0x0000000000000000000000000000000000000000", // ETH
    token1: "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512", // DAI
    volume24h: "2 ETH",
    orders: 100,
  },
];

export default async function Home() {
  const [blocks, transactions] = await Promise.all([
    getRecentBlocks(10),
    getRecentTransactions(10),
  ]);

  return (
    <div className="min-h-screen bg-black">
      <header className="border-b border-white/10">
        <div className="max-w-7xl mx-auto px-6 lg:px-8 py-8">
          <h1 className="text-4xl font-bold text-white tracking-tight">
            Explorer
          </h1>
          <p className="text-sm text-white/60 mt-2 font-mono">localhost:8545</p>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 lg:px-8 py-12">
        {/* DEX Pairs Section */}
        <div className="mb-6">
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">
                DEX Trading Pairs
              </h2>
            </div>
            <div className="divide-y divide-white/5">
              {DEX_PAIRS.map((pair, index) => {
                const token0Name = getNameTag(pair.token0) || truncateHash(pair.token0);
                const token1Name = getNameTag(pair.token1) || truncateHash(pair.token1);
                return (
                  <Link
                    key={index}
                    href={`/pair/${pair.token0}/${pair.token1}`}
                    className="block px-6 py-4 hover:bg-white/5 transition-all duration-200"
                  >
                    <div className="flex items-center justify-between gap-4">
                      <div className="flex-1">
                        <div className="text-lg font-semibold text-white mb-1">
                          {token0Name}/{token1Name}
                        </div>
                        <div className="text-xs text-white/50">
                          {pair.orders} open orders
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="text-sm text-white/90 font-mono">
                          {pair.volume24h}
                        </div>
                        <div className="text-xs text-white/50">24h volume</div>
                      </div>
                    </div>
                  </Link>
                );
              })}
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Recent Blocks */}
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">
                Recent Blocks
              </h2>
            </div>
            <div className="divide-y divide-white/5">
              {blocks.map((block) => (
                <Link
                  key={block.hash}
                  href={`/blocks/${block.hash}`}
                  className="block px-6 py-4 hover:bg-white/5 transition-all duration-200"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3 mb-2">
                        <div className="flex-shrink-0 w-8 h-8 bg-[#0052ff]/30 rounded-lg flex items-center justify-center">
                          <span className="text-sm font-medium text-[#0052ff]">
                            Bk
                          </span>
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="text-base font-semibold text-white font-mono">
                            #{block.number?.toString()}
                          </div>
                        </div>
                      </div>
                      <div className="text-xs text-white/50 font-mono pl-11">
                        {truncateHash(block.hash || "")}
                      </div>
                    </div>
                    <div className="flex-shrink-0 text-right">
                      <div className="text-sm text-white/90 font-mono mb-1">
                        {block.transactions?.length || 0} txn
                        {block.transactions?.length !== 1 ? "s" : ""}
                      </div>
                      <div className="text-xs text-white/50">
                        {formatTimestamp(block.timestamp)
                          .split(",")[1]
                          ?.trim() || formatTimestamp(block.timestamp)}
                      </div>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </div>

          {/* Recent Transactions */}
          <div className="bg-white/5 rounded-2xl border border-white/10 overflow-hidden backdrop-blur-sm">
            <div className="px-6 py-5 border-b border-white/10">
              <h2 className="text-xl font-semibold text-white">
                Recent Transactions
              </h2>
            </div>
            <div className="divide-y divide-white/5">
              {transactions.map((tx, idx) => (
                <Link
                  key={`${tx.hash}-${idx}`}
                  href={`/tx/${tx.hash}`}
                  className="block px-6 py-4 hover:bg-white/5 transition-all duration-200"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3 mb-2">
                        <div className="flex-shrink-0 w-8 h-8 bg-[#0052ff]/20 rounded-lg flex items-center justify-center">
                          <span className="text-sm font-medium text-[#0052ff]">
                            Tx
                          </span>
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="text-sm text-white/90 font-mono">
                            {truncateHash(tx.hash)}
                          </div>
                        </div>
                      </div>
                      <div className="text-xs text-white/50 font-mono pl-11">
                        {truncateHash(tx.from)} →{" "}
                        {tx.to ? truncateHash(tx.to) : "Contract"}
                      </div>
                    </div>
                    <div className="flex-shrink-0 text-right">
                      <div className="text-sm text-white/90 font-mono mb-1">
                        {formatValue(tx.value)}
                      </div>
                      <div className="text-xs text-white/50">
                        Block #{tx.blockNumber?.toString()}
                      </div>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
