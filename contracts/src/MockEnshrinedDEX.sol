// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./EnshrinedDEX.sol";

/// @title MockEnshrinedDEX
/// @notice Mock implementation of the EnshrinedDEX for testing and simulation
/// @dev This provides basic functionality so Foundry scripts can simulate successfully
contract MockEnshrinedDEX is IEnshrinedDEX {
    // Storage
    mapping(bytes32 => bool) public pairs;
    mapping(bytes32 => Order) public orders;
    mapping(address => bytes32[]) public userOrders;

    uint256 private orderNonce;

    struct Order {
        address trader;
        address tokenIn;
        address tokenOut;
        bool isBuy;
        uint256 amount;
        uint256 priceNum;
        uint256 priceDenom;
        uint8 status; // 0=open, 1=filled, 2=cancelled
    }

    // Helper to get pair ID
    function getPairId(address token0, address token1) internal pure returns (bytes32) {
        // Ensure consistent ordering
        if (token0 > token1) {
            (token0, token1) = (token1, token0);
        }
        return keccak256(abi.encodePacked(token0, token1));
    }

    function createPair(address token0, address token1) external override {
        if (token0 == address(0) && token1 == address(0)) {
            revert InvalidTokenAddress(token0);
        }
        if (token0 == token1) {
            revert InvalidTokenAddress(token0);
        }

        bytes32 pairId = getPairId(token0, token1);
        if (pairs[pairId]) {
            revert PairAlreadyExists(token0, token1, pairId);
        }

        pairs[pairId] = true;
        emit PairCreated(token0, token1, pairId);
    }

    function placeLimitOrder(
        address tokenIn,
        address tokenOut,
        bool isBuy,
        uint256 amount,
        uint256 priceNum,
        uint256 priceDenom
    ) external payable override returns (bytes32 orderId) {
        require(amount > 0, "Invalid amount");
        require(priceNum > 0 && priceDenom > 0, "Invalid price");

        // Verify pair exists
        bytes32 pairId = getPairId(tokenIn, tokenOut);
        require(pairs[pairId], "Pair does not exist");

        // Generate order ID
        orderId = keccak256(abi.encodePacked(msg.sender, orderNonce++, block.timestamp));

        // Store order
        orders[orderId] = Order({
            trader: msg.sender,
            tokenIn: tokenIn,
            tokenOut: tokenOut,
            isBuy: isBuy,
            amount: amount,
            priceNum: priceNum,
            priceDenom: priceDenom,
            status: 0 // open
        });

        // Track user orders
        userOrders[msg.sender].push(orderId);

        emit LimitOrderPlaced(
            orderId,
            msg.sender,
            tokenIn,
            tokenOut,
            isBuy,
            amount,
            priceNum,
            priceDenom
        );

        return orderId;
    }

    function cancelOrder(bytes32 orderId) external override {
        Order storage order = orders[orderId];
        require(order.trader != address(0), "Order not found");
        require(order.trader == msg.sender, "Not your order");
        require(order.status == 0, "Order not open");

        order.status = 2; // cancelled

        emit OrderCancelled(orderId, msg.sender);
    }

    function swap(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        uint256 minAmountOut
    ) external payable override returns (uint256 amountOut) {
        require(amountIn > 0, "Invalid amount");

        bytes32 pairId = getPairId(tokenIn, tokenOut);
        require(pairs[pairId], "Pair does not exist");

        // Mock: return a simple amount (90% of input for demonstration)
        amountOut = (amountIn * 90) / 100;
        require(amountOut >= minAmountOut, "Slippage exceeded");

        bytes32[] memory route = new bytes32[](1);
        route[0] = pairId;

        emit Swap(msg.sender, tokenIn, tokenOut, amountIn, amountOut, route);

        return amountOut;
    }

    function getQuote(
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) external view override returns (uint256 amountOut, bytes32[] memory route) {
        bytes32 pairId = getPairId(tokenIn, tokenOut);
        require(pairs[pairId], "Pair does not exist");

        // Mock: return 90% of input
        amountOut = (amountIn * 90) / 100;

        route = new bytes32[](1);
        route[0] = pairId;

        return (amountOut, route);
    }

    function getOrderbookDepth(
        address token0,
        address token1,
        uint256 levels
    )
        external
        view
        override
        returns (
            uint256[] memory buyPrices,
            uint256[] memory buyAmounts,
            uint256[] memory sellPrices,
            uint256[] memory sellAmounts
        )
    {
        // Return empty arrays for mock
        buyPrices = new uint256[](levels * 2);
        buyAmounts = new uint256[](levels);
        sellPrices = new uint256[](levels * 2);
        sellAmounts = new uint256[](levels);
    }

    function getPairStats(
        address token0,
        address token1
    )
        external
        view
        override
        returns (
            uint256 volume24h,
            uint256 priceNum,
            uint256 priceDenom,
            uint256 totalOrders
        )
    {
        bytes32 pairId = getPairId(token0, token1);
        require(pairs[pairId], "Pair does not exist");

        // Return mock data
        return (0, 1, 1, 0);
    }

    function getUserOrders(
        address user
    ) external view override returns (bytes32[] memory orderIds) {
        return userOrders[user];
    }

    function getOrder(
        bytes32 orderId
    )
        external
        view
        override
        returns (
            address trader,
            address tokenIn,
            address tokenOut,
            bool isBuy,
            uint256 amount,
            uint256 priceNum,
            uint256 priceDenom,
            uint8 status
        )
    {
        Order memory order = orders[orderId];
        require(order.trader != address(0), "Order not found");

        return (
            order.trader,
            order.tokenIn,
            order.tokenOut,
            order.isBuy,
            order.amount,
            order.priceNum,
            order.priceDenom,
            order.status
        );
    }
}
