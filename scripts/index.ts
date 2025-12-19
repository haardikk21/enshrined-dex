import {
  createPublicClient,
  createWalletClient,
  http,
  parseEther,
  formatEther,
  formatUnits,
  encodeAbiParameters,
  parseAbiParameters,
  encodeFunctionData,
  defineChain,
  type Hex,
  type Address,
} from "viem";
import {
  mnemonicToAccount,
  generatePrivateKey,
  privateKeyToAccount,
} from "viem/accounts";
import { readFileSync } from "fs";
import { join } from "path";

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
});

// Constants
const DEX_ADDRESS = "0x4200000000000000000000000000000000000042" as Address;
const ETH_ADDRESS = "0x0000000000000000000000000000000000000000" as Address;

// ABIs
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

  // Create clients
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

  // Check deployer balance
  const deployerBalance = await publicClient.getBalance({
    address: deployerAccount.address,
  });
  console.log(`   Deployer ETH balance: ${formatEther(deployerBalance)} ETH\n`);

  // 2. Deploy DexToken contracts
  console.log("2. Deploying DexToken contracts...");

  // Deploy USDC (6 decimals)
  const usdcConstructorArgs = encodeAbiParameters(
    parseAbiParameters("string, string, uint8"),
    ["USD Coin", "USDC", 6],
  );
  const usdcDeployData = (DEX_TOKEN_BYTECODE +
    usdcConstructorArgs.slice(2)) as Hex;

  const usdcDeployHash = await deployerClient.sendTransaction({
    data: usdcDeployData,
  });
  const usdcReceipt = await publicClient.waitForTransactionReceipt({
    hash: usdcDeployHash,
  });
  const usdcAddress = usdcReceipt.contractAddress!;
  console.log(`   USDC deployed at: ${usdcAddress}`);
  console.log(`   Explorer: http://localhost:3000/tx/${usdcDeployHash}`);

  // Deploy DAI (18 decimals)
  const daiConstructorArgs = encodeAbiParameters(
    parseAbiParameters("string, string, uint8"),
    ["Dai Stablecoin", "DAI", 18],
  );
  const daiDeployData = (DEX_TOKEN_BYTECODE +
    daiConstructorArgs.slice(2)) as Hex;

  const daiDeployHash = await deployerClient.sendTransaction({
    data: daiDeployData,
  });
  const daiReceipt = await publicClient.waitForTransactionReceipt({
    hash: daiDeployHash,
  });
  const daiAddress = daiReceipt.contractAddress!;
  console.log(`   DAI deployed at: ${daiAddress}`);
  console.log(`   Explorer: http://localhost:3000/tx/${daiDeployHash}\n`);

  // 3. Mint tokens to deployer
  console.log("3. Minting tokens to deployer...");

  const mintUsdcHash = await deployerClient.writeContract({
    address: usdcAddress,
    abi: DexToken_ABI,
    functionName: "mint",
    args: [deployerAccount.address, 1_000_000n * 10n ** 6n], // 1M USDC
  });
  await publicClient.waitForTransactionReceipt({ hash: mintUsdcHash });
  console.log("   Minted 1,000,000 USDC to deployer");
  console.log(`   Explorer: http://localhost:3000/tx/${mintUsdcHash}`);

  const mintDaiHash = await deployerClient.writeContract({
    address: daiAddress,
    abi: DexToken_ABI,
    functionName: "mint",
    args: [deployerAccount.address, 1_000_000n * 10n ** 18n], // 1M DAI
  });
  await publicClient.waitForTransactionReceipt({ hash: mintDaiHash });
  console.log("   Minted 1,000,000 DAI to deployer");
  console.log(`   Explorer: http://localhost:3000/tx/${mintDaiHash}\n`);

  // 4. Create trading pairs
  console.log("4. Creating trading pairs...");

  const createEthUsdcHash = await deployerClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "createPair",
    args: [ETH_ADDRESS, usdcAddress],
  });
  await publicClient.waitForTransactionReceipt({ hash: createEthUsdcHash });
  console.log("   Created ETH/USDC pair");
  console.log(`   Explorer: http://localhost:3000/tx/${createEthUsdcHash}`);

  const createEthDaiHash = await deployerClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "createPair",
    args: [ETH_ADDRESS, daiAddress],
  });
  await publicClient.waitForTransactionReceipt({ hash: createEthDaiHash });
  console.log("   Created ETH/DAI pair");
  console.log(`   Explorer: http://localhost:3000/tx/${createEthDaiHash}\n`);

  // 5. Place limit orders (BUY orders - offering to buy ETH with USDC/DAI)
  // For Alice's market sell (selling ETH) to match, we need BUY orders on the book
  console.log("5. Placing limit orders (BUY orders for ETH)...");

  // Place BUY order: Buy ETH at 3000 USDC per ETH
  // tokenIn = USDC (what maker pays), tokenOut = ETH (what maker receives)
  // isBuy = true, amount = USDC amount, price = 3000 USDC per 1 ETH
  const placeUsdcOrderHash = await deployerClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "placeLimitOrder",
    args: [
      usdcAddress, // tokenIn (USDC)
      ETH_ADDRESS, // tokenOut (ETH)
      true, // isBuy (buying ETH with USDC)
      30_000n * 10n ** 6n, // amount: 30,000 USDC (enough for 10 ETH)
      3000n * 10n ** 6n, // priceNum: 3000 USDC
      10n ** 18n, // priceDenom: 1 ETH
    ],
  });
  await publicClient.waitForTransactionReceipt({ hash: placeUsdcOrderHash });
  console.log("   Placed BUY order: 10 ETH at 3000 USDC/ETH");
  console.log(`   Explorer: http://localhost:3000/tx/${placeUsdcOrderHash}`);

  // Place BUY order: Buy ETH at 3000 DAI per ETH
  const placeDaiOrderHash = await deployerClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "placeLimitOrder",
    args: [
      daiAddress, // tokenIn (DAI)
      ETH_ADDRESS, // tokenOut (ETH)
      true, // isBuy (buying ETH with DAI)
      30_000n * 10n ** 18n, // amount: 30,000 DAI (enough for 10 ETH)
      3000n * 10n ** 18n, // priceNum: 3000 DAI
      10n ** 18n, // priceDenom: 1 ETH
    ],
  });
  await publicClient.waitForTransactionReceipt({ hash: placeDaiOrderHash });
  console.log("   Placed BUY order: 10 ETH at 3000 DAI/ETH");
  console.log(`   Explorer: http://localhost:3000/tx/${placeDaiOrderHash}\n`);

  // 6. Send ETH to Alice
  console.log("6. Sending ETH to Alice...");
  const sendEthHash = await deployerClient.sendTransaction({
    to: aliceAccount.address,
    value: parseEther("10"),
  });
  await publicClient.waitForTransactionReceipt({ hash: sendEthHash });
  const aliceEthBalance = await publicClient.getBalance({
    address: aliceAccount.address,
  });
  console.log(`   Alice now has ${formatEther(aliceEthBalance)} ETH`);
  console.log(`   Explorer: http://localhost:3000/tx/${sendEthHash}\n`);

  // 7. Check initial balances before swaps
  console.log("7. Checking initial balances before swaps...");

  // Alice's balances
  const aliceEthBefore = await publicClient.getBalance({
    address: aliceAccount.address,
  });
  const aliceUsdcBefore = (await publicClient.readContract({
    address: usdcAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [aliceAccount.address],
  })) as bigint;
  const aliceDaiBefore = (await publicClient.readContract({
    address: daiAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [aliceAccount.address],
  })) as bigint;

  console.log("   Alice:");
  console.log(`     ETH: ${formatEther(aliceEthBefore)} ETH`);
  console.log(`     USDC: ${formatUnits(aliceUsdcBefore, 6)}`);
  console.log(`     DAI: ${formatUnits(aliceDaiBefore, 18)}`);

  // Deployer's balances
  const deployerEthBefore = await publicClient.getBalance({
    address: deployerAccount.address,
  });
  const deployerUsdcBefore = (await publicClient.readContract({
    address: usdcAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [deployerAccount.address],
  })) as bigint;
  const deployerDaiBefore = (await publicClient.readContract({
    address: daiAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [deployerAccount.address],
  })) as bigint;

  console.log("   Deployer:");
  console.log(`     ETH: ${formatEther(deployerEthBefore)} ETH`);
  console.log(`     USDC: ${formatUnits(deployerUsdcBefore, 6)}`);
  console.log(`     DAI: ${formatUnits(deployerDaiBefore, 18)}\n`);

  // 8. Alice swaps ETH for USDC and DAI
  console.log("8. Alice swapping ETH for tokens...");

  // Swap 1 ETH for USDC
  const swapUsdcHash = await aliceClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "swap",
    args: [
      ETH_ADDRESS, // tokenIn (ETH)
      usdcAddress, // tokenOut (USDC)
      parseEther("1"), // amountIn (1 ETH)
      0n, // minAmountOut (no slippage protection for test)
    ],
    value: parseEther("1"),
  });
  await publicClient.waitForTransactionReceipt({ hash: swapUsdcHash });
  console.log("   Alice swapped 1 ETH for USDC");
  console.log(`   Explorer: http://localhost:3000/tx/${swapUsdcHash}`);

  // Swap 1 ETH for DAI
  const swapDaiHash = await aliceClient.writeContract({
    address: DEX_ADDRESS,
    abi: DEX_ABI,
    functionName: "swap",
    args: [
      ETH_ADDRESS, // tokenIn (ETH)
      daiAddress, // tokenOut (DAI)
      parseEther("1"), // amountIn (1 ETH)
      0n, // minAmountOut (no slippage protection for test)
    ],
    value: parseEther("1"),
  });
  await publicClient.waitForTransactionReceipt({ hash: swapDaiHash });
  console.log("   Alice swapped 1 ETH for DAI");
  console.log(`   Explorer: http://localhost:3000/tx/${swapDaiHash}\n`);

  // 9. Verify final balances after swaps
  console.log("9. Verifying final balances after swaps...");

  // Alice's balances
  const aliceEthAfter = await publicClient.getBalance({
    address: aliceAccount.address,
  });
  const aliceUsdcAfter = (await publicClient.readContract({
    address: usdcAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [aliceAccount.address],
  })) as bigint;
  const aliceDaiAfter = (await publicClient.readContract({
    address: daiAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [aliceAccount.address],
  })) as bigint;

  console.log("   Alice:");
  console.log(
    `     ETH: ${formatEther(aliceEthAfter)} ETH (Δ ${formatEther(aliceEthAfter - aliceEthBefore)})`,
  );
  console.log(
    `     USDC: ${formatUnits(aliceUsdcAfter, 6)} (Δ +${formatUnits(aliceUsdcAfter - aliceUsdcBefore, 6)})`,
  );
  console.log(
    `     DAI: ${formatUnits(aliceDaiAfter, 18)} (Δ +${formatUnits(aliceDaiAfter - aliceDaiBefore, 18)})`,
  );

  // Deployer's balances
  const deployerEthAfter = await publicClient.getBalance({
    address: deployerAccount.address,
  });
  const deployerUsdcAfter = (await publicClient.readContract({
    address: usdcAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [deployerAccount.address],
  })) as bigint;
  const deployerDaiAfter = (await publicClient.readContract({
    address: daiAddress,
    abi: DexToken_ABI,
    functionName: "balanceOf",
    args: [deployerAccount.address],
  })) as bigint;

  console.log("   Deployer:");
  console.log(
    `     ETH: ${formatEther(deployerEthAfter)} ETH (Δ ${formatEther(deployerEthAfter - deployerEthBefore)})`,
  );
  console.log(
    `     USDC: ${formatUnits(deployerUsdcAfter, 6)} (Δ ${formatUnits(deployerUsdcAfter - deployerUsdcBefore, 6)})`,
  );
  console.log(
    `     DAI: ${formatUnits(deployerDaiAfter, 18)} (Δ ${formatUnits(deployerDaiAfter - deployerDaiBefore, 18)})\n`,
  );

  // Verify results
  console.log("=== Test Results ===");
  if (aliceUsdcAfter > 0n && aliceDaiAfter > 0n) {
    console.log("✅ SUCCESS: All token transfers working correctly!");
    console.log(
      `   Alice received: ${formatUnits(aliceUsdcAfter - aliceUsdcBefore, 6)} USDC, ${formatUnits(aliceDaiAfter - aliceDaiBefore, 18)} DAI`,
    );
    console.log(
      `   Alice spent: ~${formatEther(aliceEthBefore - aliceEthAfter)} ETH (including gas)`,
    );
    console.log(
      `   Deployer received: ${formatEther(deployerEthAfter - deployerEthBefore)} ETH from swaps`,
    );
  } else {
    console.log("❌ FAILURE: Alice did not receive expected tokens");
    if (aliceUsdcAfter === 0n) console.log("   - No USDC received");
    if (aliceDaiAfter === 0n) console.log("   - No DAI received");
  }
}

main().catch(console.error);
