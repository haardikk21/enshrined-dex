// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "solmate/tokens/ERC20.sol";
import "../src/EnshrinedDEX.sol";
import "../src/MockEnshrinedDEX.sol";

/// @notice Mock ERC20 token for testing
contract MockERC20 is ERC20 {
    constructor(
        string memory name,
        string memory symbol,
        uint8 decimals
    ) ERC20(name, symbol, decimals) {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @title SetupDEX
/// @notice Script to deploy mock tokens and initialize the DEX with liquidity
contract SetupDEX is Script {
    // Hardcoded private key
    uint256 constant DEPLOYER_PRIVATE_KEY =
        0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;

    // DEX predeploy address
    address constant DEX_ADDRESS = 0x4200000000000000000000000000000000000042;

    // ETH address (address zero)
    address constant ETH = address(0);

    function run() external {
        // Get deployer address from private key
        address deployer = vm.addr(DEPLOYER_PRIVATE_KEY);
        console.log("Deployer address:", deployer);

        // Use vm.etch to place MockEnshrinedDEX bytecode at the predeploy address
        // This tricks Foundry into thinking a contract exists there during simulation
        console.log("\n=== Setting up Mock DEX at predeploy address ===");
        MockEnshrinedDEX mockImpl = new MockEnshrinedDEX();
        vm.etch(DEX_ADDRESS, address(mockImpl).code);
        console.log("Mock DEX bytecode placed at:", DEX_ADDRESS);

        vm.startBroadcast(DEPLOYER_PRIVATE_KEY);

        // Deploy mock tokens
        console.log("\n=== Deploying Mock Tokens ===");

        MockERC20 usdc = new MockERC20("USD Coin", "USDC", 6);
        console.log("USDC deployed at:", address(usdc));

        MockERC20 base = new MockERC20("Base", "BASE", 18);
        console.log("BASE deployed at:", address(base));

        MockERC20 sol = new MockERC20("Solana", "SOL", 9);
        console.log("SOL deployed at:", address(sol));

        // Mint 1 million of each token to deployer
        console.log("\n=== Minting Tokens ===");

        usdc.mint(deployer, 1_000_000 * 10 ** 6); // 1M USDC (6 decimals)
        console.log("Minted 1,000,000 USDC to deployer");

        base.mint(deployer, 1_000_000 * 10 ** 18); // 1M BASE (18 decimals)
        console.log("Minted 1,000,000 BASE to deployer");

        sol.mint(deployer, 1_000_000 * 10 ** 9); // 1M SOL (9 decimals)
        console.log("Minted 1,000,000 SOL to deployer");

        // Get DEX interface at predeploy address
        IEnshrinedDEX dex = IEnshrinedDEX(DEX_ADDRESS);

        // Create pairs
        console.log("\n=== Creating Pairs ===");

        dex.createPair(ETH, address(usdc));
        console.log("Created ETH/USDC pair");

        dex.createPair(ETH, address(base));
        console.log("Created ETH/BASE pair");

        dex.createPair(ETH, address(sol));
        console.log("Created ETH/SOL pair");

        // Place limit orders
        // Price ratios: 1 ETH = 3000 USDC, 1 BASE = 2 USDC (equiv in ETH), 1 SOL = 120 USDC (equiv in ETH)
        // That means: 1 ETH = 1500 BASE, 1 ETH = 25 SOL

        console.log("\n=== Placing Limit Orders ===");

        // ETH/USDC orders (1 ETH = 3000 USDC)
        // Buy orders (buying ETH with USDC)
        placeETHUSDCBuyOrders(dex, address(usdc));

        // Sell orders (selling ETH for USDC)
        placeETHUSDCSellOrders(dex, address(usdc));

        // ETH/BASE orders (1 ETH = 1500 BASE, i.e., 1 BASE = 0.000666... ETH)
        placeETHBASEOrders(dex, address(base));

        // ETH/SOL orders (1 ETH = 25 SOL, i.e., 1 SOL = 0.04 ETH)
        placeETHSOLOrders(dex, address(sol));

        vm.stopBroadcast();

        console.log("\n=== Setup Complete ===");
        console.log("DEX Address:", DEX_ADDRESS);
        console.log("USDC:", address(usdc));
        console.log("BASE:", address(base));
        console.log("SOL:", address(sol));
    }

    function placeETHUSDCBuyOrders(IEnshrinedDEX dex, address usdc) internal {
        console.log("\nPlacing ETH/USDC buy orders (buying ETH with USDC):");

        // Buy order at 2950 USDC per ETH (bid below market)
        // Price = amount of USDC per 1 ETH
        // priceNum = 2950 * 10^6 (USDC units), priceDenom = 1 * 10^18 (ETH units)
        dex.placeLimitOrder(
            usdc, // tokenIn (paying with USDC)
            ETH, // tokenOut (buying ETH)
            true, // isBuy
            5900 * 10 ** 6, // amount: 5900 USDC (enough for ~2 ETH)
            2950 * 10 ** 6, // priceNum (2950 USDC)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Buy 2 ETH at 2950 USDC/ETH");

        // Buy order at 2900 USDC per ETH
        dex.placeLimitOrder(
            usdc,
            ETH,
            true,
            8700 * 10 ** 6, // 8700 USDC (enough for ~3 ETH)
            2900 * 10 ** 6,
            1 * 10 ** 18
        );
        console.log("  - Buy 3 ETH at 2900 USDC/ETH");

        // Buy order at 2850 USDC per ETH
        dex.placeLimitOrder(
            usdc,
            ETH,
            true,
            14250 * 10 ** 6, // 14250 USDC (enough for ~5 ETH)
            2850 * 10 ** 6,
            1 * 10 ** 18
        );
        console.log("  - Buy 5 ETH at 2850 USDC/ETH");
    }

    function placeETHUSDCSellOrders(IEnshrinedDEX dex, address usdc) internal {
        console.log("\nPlacing ETH/USDC sell orders (selling ETH for USDC):");

        // Sell order at 3050 USDC per ETH (ask above market)
        dex.placeLimitOrder{value: 2 ether}(
            ETH, // tokenIn (paying with ETH)
            usdc, // tokenOut (receiving USDC)
            false, // isSell
            2 * 10 ** 18, // amount: 2 ETH
            3050 * 10 ** 6, // priceNum (3050 USDC)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Sell 2 ETH at 3050 USDC/ETH");

        // Sell order at 3100 USDC per ETH
        dex.placeLimitOrder{value: 3 ether}(
            ETH,
            usdc,
            false,
            3 * 10 ** 18, // 3 ETH
            3100 * 10 ** 6,
            1 * 10 ** 18
        );
        console.log("  - Sell 3 ETH at 3100 USDC/ETH");

        // Sell order at 3150 USDC per ETH
        dex.placeLimitOrder{value: 5 ether}(
            ETH,
            usdc,
            false,
            5 * 10 ** 18, // 5 ETH
            3150 * 10 ** 6,
            1 * 10 ** 18
        );
        console.log("  - Sell 5 ETH at 3150 USDC/ETH");
    }

    function placeETHBASEOrders(
        IEnshrinedDEX dex,
        address base_token
    ) internal {
        console.log("\nPlacing ETH/BASE orders (1 ETH = 1500 BASE):");

        // Sell ETH for BASE at 1520 BASE per ETH (slightly above 1500)
        dex.placeLimitOrder{value: 2 ether}(
            ETH,
            base_token,
            false,
            2 * 10 ** 18, // 2 ETH
            1520 * 10 ** 18, // priceNum (1520 BASE)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Sell 2 ETH at 1520 BASE/ETH");

        // Buy ETH with BASE at 1480 BASE per ETH (slightly below 1500)
        dex.placeLimitOrder(
            base_token,
            ETH,
            true,
            4440 * 10 ** 18, // 4440 BASE (enough for ~3 ETH)
            1480 * 10 ** 18, // priceNum (1480 BASE)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Buy 3 ETH at 1480 BASE/ETH");

        // Sell ETH for BASE at 1550 BASE per ETH
        dex.placeLimitOrder{value: 3 ether}(
            ETH,
            base_token,
            false,
            3 * 10 ** 18, // 3 ETH
            1550 * 10 ** 18, // priceNum (1550 BASE)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Sell 3 ETH at 1550 BASE/ETH");
    }

    function placeETHSOLOrders(IEnshrinedDEX dex, address sol_token) internal {
        console.log("\nPlacing ETH/SOL orders (1 ETH = 25 SOL):");

        // Sell ETH for SOL at 25.5 SOL per ETH (slightly above 25)
        dex.placeLimitOrder{value: 2 ether}(
            ETH,
            sol_token,
            false,
            2 * 10 ** 18, // 2 ETH
            255 * 10 ** 8, // priceNum (25.5 SOL, 9 decimals)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Sell 2 ETH at 25.5 SOL/ETH");

        // Buy ETH with SOL at 24.5 SOL per ETH (slightly below 25)
        dex.placeLimitOrder(
            sol_token,
            ETH,
            true,
            735 * 10 ** 8, // 73.5 SOL (enough for ~3 ETH)
            245 * 10 ** 8, // priceNum (24.5 SOL)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Buy 3 ETH at 24.5 SOL/ETH");

        // Sell ETH for SOL at 26 SOL per ETH
        dex.placeLimitOrder{value: 3 ether}(
            ETH,
            sol_token,
            false,
            3 * 10 ** 18, // 3 ETH
            26 * 10 ** 9, // priceNum (26 SOL)
            1 * 10 ** 18 // priceDenom (1 ETH)
        );
        console.log("  - Sell 3 ETH at 26 SOL/ETH");
    }
}
