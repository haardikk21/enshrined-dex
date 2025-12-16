import { createPublicClient, http, Block, Transaction, TransactionReceipt, formatEther } from 'viem'
import { localChain } from './wagmi'

const client = createPublicClient({
  chain: localChain,
  transport: http('http://localhost:8545'),
})

export async function getLatestBlockNumber(): Promise<bigint> {
  return await client.getBlockNumber()
}

export async function getBlock(blockNumberOrHash: bigint | string): Promise<Block | null> {
  try {
    if (typeof blockNumberOrHash === 'string') {
      return await client.getBlock({ blockHash: blockNumberOrHash as `0x${string}` })
    }
    return await client.getBlock({ blockNumber: blockNumberOrHash })
  } catch (error) {
    console.error('Error fetching block:', error)
    return null
  }
}

export async function getRecentBlocks(count: number = 10): Promise<Block[]> {
  const latestBlockNumber = await getLatestBlockNumber()
  const blocks: Block[] = []

  for (let i = 0; i < count; i++) {
    const blockNumber = latestBlockNumber - BigInt(i)
    if (blockNumber < 0n) break

    const block = await getBlock(blockNumber)
    if (block) blocks.push(block)
  }

  return blocks
}

export async function getTransaction(hash: string): Promise<Transaction | null> {
  try {
    return await client.getTransaction({ hash: hash as `0x${string}` })
  } catch (error) {
    console.error('Error fetching transaction:', error)
    return null
  }
}

export async function getTransactionReceipt(hash: string): Promise<TransactionReceipt | null> {
  try {
    return await client.getTransactionReceipt({ hash: hash as `0x${string}` })
  } catch (error) {
    console.error('Error fetching transaction receipt:', error)
    return null
  }
}

export async function getRecentTransactions(count: number = 10): Promise<Transaction[]> {
  const latestBlockNumber = await getLatestBlockNumber()
  const transactions: Transaction[] = []

  let blocksChecked = 0
  let i = 0

  while (transactions.length < count && blocksChecked < 50) {
    const blockNumber = latestBlockNumber - BigInt(i)
    if (blockNumber < 0n) break

    const block = await getBlock(blockNumber)
    if (block && block.transactions) {
      for (const txHash of block.transactions.slice(0, count - transactions.length)) {
        if (typeof txHash === 'string') {
          const tx = await getTransaction(txHash)
          if (tx) transactions.push(tx)
        }
      }
    }

    blocksChecked++
    i++
  }

  return transactions
}

export function truncateHash(hash: string, start: number = 6, end: number = 4): string {
  if (!hash) return ''
  return `${hash.slice(0, start + 2)}...${hash.slice(-end)}`
}

export function formatTimestamp(timestamp: bigint): string {
  const date = new Date(Number(timestamp) * 1000)
  return date.toLocaleString()
}

export function formatGas(gas: bigint): string {
  return gas.toLocaleString()
}

export function formatValue(value: bigint): string {
  return `${formatEther(value)} ETH`
}
