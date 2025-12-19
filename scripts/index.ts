import {
  createPublicClient,
  createWalletClient,
  http,
  parseEther,
  formatEther,
  formatUnits,
  encodeAbiParameters,
  parseAbiParameters,
  defineChain,
  type Hex,
  type Address,
  type WalletClient,
  type PublicClient,
} from "viem";
import {
  mnemonicToAccount,
  generatePrivateKey,
  privateKeyToAccount,
} from "viem/accounts";
import { readFileSync } from "fs";
import { join } from "path";

// ============================================================================
// Constants and Configuration
// ============================================================================

const DEX_ADDRESS = "0x4200000000000000000000000000000000000042" as Address;
const ETH_ADDRESS = "0x0000000000000000000000000000000000000000" as Address;

// Load DexToken artifact
const dexTokenArtifact = JSON.parse(
  readFileSync(
    join(__dirname, "../contracts/out/DexToken.sol/DexToken.json"),
    "utf-8",
  ),
);
const DEX_TOKEN_BYTECODE = dexTokenArtifact.bytecode.object as Hex;

// Define custom chain for reth dev mode
const rethDev = defineChain({
  id: 1337,
  name: "Reth Dev",
  nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
  rpcUrls: {
    default: { http: ["http://localhost:8545"] },
  },
  blockTime: 500,
});

// ============================================================================
// ABIs
// ============================================================================

const DexToken_ABI = [
  {
    type: "constructor",
    inputs: [
      { name: "name", type: "string" },
      { name: "symbol", type: "string" },
      { name: "decimals_", type: "uint8" },
    ],
  },
  {
    type: "function",
    name: "mint",
    inputs: [
      { name: "to", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "approve",
    inputs: [
      { name: "spender", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [{ type: "bool" }],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "balanceOf",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ type: "uint256" }],
    stateMutability: "view",
  },
] as const;

const DEX_ABI = [
  {
    type: "function",
    name: "createPair",
    inputs: [
      { name: "token0", type: "address" },
      { name: "token1", type: "address" },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "placeLimitOrder",
    inputs: [
      { name: "tokenIn", type: "address" },
      { name: "tokenOut", type: "address" },
      { name: "isBuy", type: "bool" },
      { name: "amount", type: "uint256" },
      { name: "priceNum", type: "uint256" },
      { name: "priceDenom", type: "uint256" },
    ],
    outputs: [{ type: "bytes32" }],
    stateMutability: "payable",
  },
  {
    type: "function",
    name: "swap",
    inputs: [
      { name: "tokenIn", type: "address" },
      { name: "tokenOut", type: "address" },
      { name: "amountIn", type: "uint256" },
      { name: "minAmountOut", type: "uint256" },
    ],
    outputs: [{ type: "uint256" }],
    stateMutability: "payable",
  },
] as const;

// ============================================================================
// Helper Functions
// ============================================================================

interface TokenConfig {
  name: string;
  symbol: string;
  decimals: number;
}

interface DeployedToken {
  address: Address;
  config: TokenConfig;
  deployHash: Hex;
}

/**
 * Deploy a DexToken contract
 */
async function deployToken(
  client: WalletClient,
  publicClient: PublicClient,
  config: TokenConfig,
): Promise<DeployedToken> {
  const constructorArgs = encodeAbiParameters(
    parseAbiParameters("string, string, uint8"),
    [config.name, config.symbol, config.decimals],
  );
  const deployData = (DEX_TOKEN_BYTECODE + constructorArgs.slice(2)) as Hex;

  const deployHash = await client.sendTransaction({ data: deployData });
  const receipt = await publicClient.waitForTransactionReceipt({
    hash: deployHash,
  });

  return {
    address: receipt.contractAddress!,
    config,
    deployHash,
  };
}

interface OrderParams {
  tokenIn: Address;
  tokenOut: Address;
  isBuy: boolean;
  priceRange: { min: number; max: number };
  orderSize: number; // in base token units (e.g., ETH)
  count: number;
  tokenInDecimals: number;
}

/**
 * Place multiple limit orders across a price range
 */
async function placeOrderBatch(
  client: WalletClient,
  publicClient: PublicClient,
  params: OrderParams,
): Promise<void> {
  const {
    tokenIn,
    tokenOut,
    isBuy,
    priceRange,
    orderSize,
    count,
    tokenInDecimals,
  } = params;
  const hashes: Hex[] = [];

  for (let i = 0; i < count; i++) {
    const price =
      priceRange.min + ((priceRange.max - priceRange.min) * i) / (count - 1);
    const priceNum = BigInt(Math.floor(price * Math.pow(10, tokenInDecimals)));
    const priceDenom = 10n ** 18n; // Assuming ETH (18 decimals) as base

    let amount: bigint;
    let value: bigint = 0n;

    if (isBuy) {
      // Buying: amount is in tokenIn (what we're paying)
      amount = BigInt(
        Math.floor(orderSize * price * Math.pow(10, tokenInDecimals)),
      );
    } else {
      // Selling: amount is in tokenIn (what we're selling)
      amount = BigInt(Math.floor(orderSize * 1e18));
      // If selling ETH, must send value
      if (tokenIn === ETH_ADDRESS) {
        value = amount;
      }
    }

    const hash = await client.writeContract({
      address: DEX_ADDRESS,
      abi: DEX_ABI,
      functionName: "placeLimitOrder",
      args: [tokenIn, tokenOut, isBuy, amount, priceNum, priceDenom],
      value,
    });
    hashes.push(hash);
  }

  // Wait for all confirmations in parallel
  await Promise.all(
    hashes.map((hash) => publicClient.waitForTransactionReceipt({ hash })),
  );
}

interface TokenBalances {
  eth: bigint;
  usdc: bigint;
  dai: bigint;
}

/**
 * Get token balances for an address
 */
async function getBalances(
  publicClient: PublicClient,
  address: Address,
  usdcAddress: Address,
  daiAddress: Address,
): Promise<TokenBalances> {
  const [eth, usdc, dai] = await Promise.all([
    publicClient.getBalance({ address }),
    publicClient.readContract({
      address: usdcAddress,
      abi: DexToken_ABI,
      functionName: "balanceOf",
      args: [address],
    }) as Promise<bigint>,
    publicClient.readContract({
      address: daiAddress,
      abi: DexToken_ABI,
      functionName: "balanceOf",
      args: [address],
    }) as Promise<bigint>,
  ]);

  return { eth, usdc, dai };
}

/**
 * Print balances in a readable format
 */
function printBalances(name: string, balances: TokenBalances): void {
  console.log(`   ${name}:`);
  console.log(`     ETH:  ${formatEther(balances.eth)} ETH`);
  console.log(`     USDC: ${formatUnits(balances.usdc, 6)}`);
  console.log(`     DAI:  ${formatUnits(balances.dai, 18)}`);
}

// ============================================================================
// Main Function
// ============================================================================

async function main() {
  console.log("=== Enshrined DEX End-to-End Test ===\n");

  // 1. Initialize wallets
  console.log("1. Initializing wallets...");
  const deployerAccount = mnemonicToAccount(
    "test test test test test test test test test test test junk",
  );
  const alicePrivateKey = generatePrivateKey();
  const aliceAccount = privateKeyToAccount(alicePrivateKey);

  console.log(`   Deployer: ${deployerAccount.address}`);
  console.log(`   Alice:    ${aliceAccount.address}`);

  const publicClient = createPublicClient({
    chain: rethDev,
    transport: http(),
  });

  const deployerClient = createWalletClient({
    account: deployerAccount,
    chain: rethDev,
    transport: http(),
  });

  const aliceClient = createWalletClient({
    account: aliceAccount,
    chain: rethDev,
    transport: http(),
  });

  const deployerBalance = await publicClient.getBalance({
    address: deployerAccount.address,
  });
  console.log(`   Deployer ETH balance: ${formatEther(deployerBalance)} ETH\n`);

  // 2. Deploy tokens sequentially (nonce management)
  console.log("2. Deploying DexToken contracts...");
  const usdc = await deployToken(deployerClient, publicClient, {
    name: "USD Coin",
    symbol: "USDC",
    decimals: 6,
  });
  const dai = await deployToken(deployerClient, publicClient, {
    name: "Dai Stablecoin",
    symbol: "DAI",
    decimals: 18,
  });

  console.log(`   USDC deployed at: ${usdc.address}`);
  console.log(`   Explorer: http://localhost:3000/tx/${usdc.deployHash}`);
  console.log(`   DAI deployed at: ${dai.address}`);
  console.log(`   Explorer: http://localhost:3000/tx/${dai.deployHash}\n`);

  // 3. Setup: Mint tokens, create pairs, fund Alice
  console.log("3. Setting up DEX (minting, creating pairs, funding Alice)...");

  // Mint USDC
  await publicClient.waitForTransactionReceipt({
    hash: await deployerClient.writeContract({
      address: usdc.address,
      abi: DexToken_ABI,
      functionName: "mint",
      args: [deployerAccount.address, 1_000_000n * 10n ** 6n],
    }),
  });

  // Mint DAI
  await publicClient.waitForTransactionReceipt({
    hash: await deployerClient.writeContract({
      address: dai.address,
      abi: DexToken_ABI,
      functionName: "mint",
      args: [deployerAccount.address, 1_000_000n * 10n ** 18n],
    }),
  });

  // Create ETH/USDC pair
  await publicClient.waitForTransactionReceipt({
    hash: await deployerClient.writeContract({
      address: DEX_ADDRESS,
      abi: DEX_ABI,
      functionName: "createPair",
      args: [ETH_ADDRESS, usdc.address],
    }),
  });

  // Create ETH/DAI pair
  await publicClient.waitForTransactionReceipt({
    hash: await deployerClient.writeContract({
      address: DEX_ADDRESS,
      abi: DEX_ABI,
      functionName: "createPair",
      args: [ETH_ADDRESS, dai.address],
    }),
  });

  // Fund Alice
  await publicClient.waitForTransactionReceipt({
    hash: await deployerClient.sendTransaction({
      to: aliceAccount.address,
      value: parseEther("10"),
    }),
  });

  console.log("   ✓ All setup complete!");
  console.log(`   View transactions: http://localhost:3000\n`);

  // 4. Place buy orders (orders to buy ETH)
  console.log("4. Placing buy orders (100 USDC orders, 100 DAI orders)...");

  await placeOrderBatch(deployerClient, publicClient, {
    tokenIn: usdc.address,
    tokenOut: ETH_ADDRESS,
    isBuy: true,
    priceRange: { min: 2995, max: 3000 },
    orderSize: 0.1, // 0.1 ETH per order
    count: 100,
    tokenInDecimals: 6, // USDC decimals
  });

  await placeOrderBatch(deployerClient, publicClient, {
    tokenIn: dai.address,
    tokenOut: ETH_ADDRESS,
    isBuy: true,
    priceRange: { min: 2995, max: 3000 },
    orderSize: 0.1,
    count: 100,
    tokenInDecimals: 18, // DAI decimals
  });

  console.log("   ✓ Placed 200 buy orders\n");

  // 5. Place sell orders (orders to sell ETH)
  console.log("5. Placing sell orders (100 USDC orders, 100 DAI orders)...");

  await placeOrderBatch(deployerClient, publicClient, {
    tokenIn: ETH_ADDRESS,
    tokenOut: usdc.address,
    isBuy: false,
    priceRange: { min: 3002, max: 3007 },
    orderSize: 0.1,
    count: 100,
    tokenInDecimals: 6, // USDC decimals (for price)
  });

  await placeOrderBatch(deployerClient, publicClient, {
    tokenIn: ETH_ADDRESS,
    tokenOut: dai.address,
    isBuy: false,
    priceRange: { min: 3002, max: 3007 },
    orderSize: 0.1,
    count: 100,
    tokenInDecimals: 18, // DAI decimals (for price)
  });

  console.log("   ✓ Placed 200 sell orders\n");

  // 6. Check balances before swaps
  console.log("6. Checking balances before swaps...");
  const aliceBalancesBefore = await getBalances(
    publicClient,
    aliceAccount.address,
    usdc.address,
    dai.address,
  );
  const deployerBalancesBefore = await getBalances(
    publicClient,
    deployerAccount.address,
    usdc.address,
    dai.address,
  );

  printBalances("Alice", aliceBalancesBefore);
  printBalances("Deployer", deployerBalancesBefore);
  console.log();

  // Wait for user input before demo
  console.log("=== SETUP COMPLETE ===");
  console.log("\nOrderbook is ready with:");
  console.log("  • 100 USDC buy orders at 2999-3001");
  console.log("  • 100 DAI buy orders at 2999-3001");
  console.log("  • 100 USDC sell orders at 3005-3007");
  console.log("  • 100 DAI sell orders at 3005-3007");
  console.log(
    `\nView orderbook: http://localhost:3000/pair/${ETH_ADDRESS}/${usdc.address}`,
  );
  console.log("\nPress Enter to execute demo swap...");
  await new Promise<void>((resolve) => {
    process.stdin.once("data", () => resolve());
  });

  // 7. Alice swaps ETH for tokens (matching against many orders)
  console.log(
    "\n7. Alice swapping ETH for tokens (matching against 100s of orders)...",
  );

  const swapUsdcHash = await aliceClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "swap",
    args: [ETH_ADDRESS, usdc.address, parseEther("1"), 0n],
    value: parseEther("1"),
  });
  await publicClient.waitForTransactionReceipt({ hash: swapUsdcHash });

  const swapDaiHash = await aliceClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "swap",
    args: [ETH_ADDRESS, dai.address, parseEther("1"), 0n],
    value: parseEther("1"),
  });
  await publicClient.waitForTransactionReceipt({ hash: swapDaiHash });

  console.log("   ✓ Alice swapped 1 ETH for USDC");
  console.log(`     Explorer: http://localhost:3000/tx/${swapUsdcHash}`);
  console.log("   ✓ Alice swapped 1 ETH for DAI");
  console.log(`     Explorer: http://localhost:3000/tx/${swapDaiHash}\n`);

  // 8. Check final balances
  console.log("8. Checking final balances...");
  const aliceBalancesAfter = await getBalances(
    publicClient,
    aliceAccount.address,
    usdc.address,
    dai.address,
  );
  const deployerBalancesAfter = await getBalances(
    publicClient,
    deployerAccount.address,
    usdc.address,
    dai.address,
  );

  printBalances("Alice", aliceBalancesAfter);
  printBalances("Deployer", deployerBalancesAfter);
  console.log();

  // Summary
  console.log("=== Summary ===");
  console.log(`Alice traded 2 ETH and received:`);
  console.log(
    `  USDC: ${formatUnits(aliceBalancesAfter.usdc - aliceBalancesBefore.usdc, 6)}`,
  );
  console.log(
    `  DAI:  ${formatUnits(aliceBalancesAfter.dai - aliceBalancesBefore.dai, 18)}`,
  );
  console.log();
  console.log("View full orderbook visualization:");
  console.log(
    `  ETH/USDC: http://localhost:3000/pair/${ETH_ADDRESS}/${usdc.address}`,
  );
  console.log(
    `  ETH/DAI:  http://localhost:3000/pair/${ETH_ADDRESS}/${dai.address}`,
  );
  console.log("\n✅ Test completed successfully!");
}

main().catch(console.error);
