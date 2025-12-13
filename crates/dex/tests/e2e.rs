//! End-to-end tests for the DEX orderbook.

use dex::{
    Address, DexConfig, OrderSide, Pair, PoolManager, Price, U256,
};

// Token addresses for testing
fn eth() -> Address {
    Address::ZERO
}

fn usdc() -> Address {
    Address::repeat_byte(0x01)
}

fn wbtc() -> Address {
    Address::repeat_byte(0x02)
}

fn dai() -> Address {
    Address::repeat_byte(0x03)
}

fn link() -> Address {
    Address::repeat_byte(0x04)
}

// Trader addresses
fn alice() -> Address {
    Address::repeat_byte(0xAA)
}

fn bob() -> Address {
    Address::repeat_byte(0xBB)
}

fn charlie() -> Address {
    Address::repeat_byte(0xCC)
}

// Helper to create amounts with decimals
fn eth_amount(eth: u64) -> U256 {
    U256::from(eth) * U256::from(10u64.pow(18))
}

fn usdc_amount(usdc: u64) -> U256 {
    U256::from(usdc) * U256::from(10u64.pow(6))
}

fn btc_amount(btc: u64) -> U256 {
    U256::from(btc) * U256::from(10u64.pow(8))
}

/// Set up a pool manager with common trading pairs and liquidity.
fn setup_market() -> PoolManager {
    let mut pm = PoolManager::new();

    // Create pairs
    pm.create_pair(eth(), usdc()).unwrap();   // ETH/USDC
    pm.create_pair(wbtc(), usdc()).unwrap();  // WBTC/USDC
    pm.create_pair(eth(), wbtc()).unwrap();   // ETH/WBTC
    pm.create_pair(dai(), usdc()).unwrap();   // DAI/USDC (stablecoin pair)
    pm.create_pair(link(), eth()).unwrap();   // LINK/ETH

    pm
}

/// Add liquidity to the ETH/USDC pair.
fn add_eth_usdc_liquidity(pm: &mut PoolManager) {
    let maker = alice();

    // Add sell orders (selling ETH for USDC) at various prices
    // Price: USDC per ETH
    for (price, amount) in [
        (2000u128, 10u64),  // 10 ETH at $2000
        (2010u128, 10u64),  // 10 ETH at $2010
        (2020u128, 10u64),  // 10 ETH at $2020
        (2050u128, 5u64),   // 5 ETH at $2050
        (2100u128, 5u64),   // 5 ETH at $2100
    ] {
        pm.place_limit_order(
            eth(),
            usdc(),
            maker,
            OrderSide::Sell,
            Price::from_u128(price * 10u128.pow(6), 10u128.pow(18)), // USDC (6 dec) per ETH (18 dec)
            eth_amount(amount),
        )
        .unwrap();
    }

    // Add buy orders (buying ETH with USDC) at various prices
    for (price, amount) in [
        (1990u128, 10u64),  // Buy 10 ETH at $1990
        (1980u128, 10u64),  // Buy 10 ETH at $1980
        (1970u128, 10u64),  // Buy 10 ETH at $1970
        (1950u128, 5u64),   // Buy 5 ETH at $1950
        (1900u128, 5u64),   // Buy 5 ETH at $1900
    ] {
        pm.place_limit_order(
            eth(),
            usdc(),
            maker,
            OrderSide::Buy,
            Price::from_u128(price * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(amount),
        )
        .unwrap();
    }
}

/// Add liquidity to the WBTC/USDC pair.
fn add_wbtc_usdc_liquidity(pm: &mut PoolManager) {
    let maker = alice();

    // Sell orders for WBTC
    for (price, amount) in [
        (40000u128, 1u64),  // 1 BTC at $40,000
        (40500u128, 1u64),  // 1 BTC at $40,500
        (41000u128, 2u64),  // 2 BTC at $41,000
    ] {
        pm.place_limit_order(
            wbtc(),
            usdc(),
            maker,
            OrderSide::Sell,
            Price::from_u128(price * 10u128.pow(6), 10u128.pow(8)), // USDC per BTC
            btc_amount(amount),
        )
        .unwrap();
    }

    // Buy orders for WBTC
    for (price, amount) in [
        (39500u128, 1u64),
        (39000u128, 1u64),
        (38500u128, 2u64),
    ] {
        pm.place_limit_order(
            wbtc(),
            usdc(),
            maker,
            OrderSide::Buy,
            Price::from_u128(price * 10u128.pow(6), 10u128.pow(8)),
            btc_amount(amount),
        )
        .unwrap();
    }
}

/// Add liquidity to ETH/WBTC pair for direct trading.
#[allow(dead_code)]
fn add_eth_wbtc_liquidity(pm: &mut PoolManager) {
    let maker = alice();

    // 1 WBTC = 20 ETH approximately
    // Sell ETH for WBTC
    pm.place_limit_order(
        eth(),
        wbtc(),
        maker,
        OrderSide::Sell,
        Price::from_u128(5 * 10u128.pow(8), 100 * 10u128.pow(18)), // 0.05 BTC per ETH
        eth_amount(100),
    )
    .unwrap();

    // Buy ETH with WBTC
    pm.place_limit_order(
        eth(),
        wbtc(),
        maker,
        OrderSide::Buy,
        Price::from_u128(48 * 10u128.pow(6), 10u128.pow(18)), // 0.048 BTC per ETH
        eth_amount(100),
    )
    .unwrap();
}

// ============================================================================
// Test Cases
// ============================================================================

#[test]
fn test_setup_pairs() {
    let pm = setup_market();

    // Verify all pairs were created
    let pairs = pm.pairs();
    assert_eq!(pairs.len(), 5);

    assert!(pm.pair_exists(eth(), usdc()));
    assert!(pm.pair_exists(wbtc(), usdc()));
    assert!(pm.pair_exists(eth(), wbtc()));
    assert!(pm.pair_exists(dai(), usdc()));
    assert!(pm.pair_exists(link(), eth()));

    // Order shouldn't matter for existence check
    assert!(pm.pair_exists(usdc(), eth()));
}

#[test]
fn test_limit_order_no_match() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Place a buy order below the best ask - should not match
    let (_order_id, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            bob(),
            OrderSide::Buy,
            Price::from_u128(1995 * 10u128.pow(6), 10u128.pow(18)), // $1995
            eth_amount(5),
        )
        .unwrap();

    assert!(result.fills.is_empty());
    assert_eq!(result.remaining_amount, eth_amount(5));
    assert!(!result.fully_filled);
}

#[test]
fn test_limit_order_partial_match() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Place a buy order that matches the best ask partially
    // Best ask is 10 ETH at $2000, we want 15 ETH at $2000
    let (_, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            bob(),
            OrderSide::Buy,
            Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(15),
        )
        .unwrap();

    // Should match the 10 ETH at $2000, leaving 5 ETH unfilled
    assert_eq!(result.fills.len(), 1);
    assert_eq!(result.fills[0].base_amount, eth_amount(10));
    assert_eq!(result.remaining_amount, eth_amount(5));
    assert!(!result.fully_filled);
}

#[test]
fn test_limit_order_full_match_multiple_levels() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Place a buy order that sweeps multiple price levels
    // We have: 10 @ $2000, 10 @ $2010, 10 @ $2020
    // Buy 25 ETH at $2020 should match all three levels
    let (_, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            bob(),
            OrderSide::Buy,
            Price::from_u128(2020 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(25),
        )
        .unwrap();

    // Should fill 10 + 10 + 5 = 25 ETH across 3 price levels
    assert_eq!(result.fills.len(), 3);
    assert!(result.fully_filled);
    assert_eq!(result.remaining_amount, U256::ZERO);

    // Verify price-time priority (cheapest first)
    // First fill should be at $2000
    let first_fill_price: u128 = result.fills[0].price.numerator.try_into().unwrap();
    assert_eq!(first_fill_price, 2000 * 10u128.pow(6));
}

#[test]
fn test_market_order_buy() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Market buy 5 ETH
    let result = pm
        .place_market_order(eth(), usdc(), bob(), OrderSide::Buy, eth_amount(5))
        .unwrap();

    assert_eq!(result.fills.len(), 1);
    assert_eq!(result.fills[0].base_amount, eth_amount(5));
    assert!(result.fully_filled);
}

#[test]
fn test_market_order_sell() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Market sell 5 ETH
    let result = pm
        .place_market_order(eth(), usdc(), bob(), OrderSide::Sell, eth_amount(5))
        .unwrap();

    // Should match against the best bid at $1990
    assert_eq!(result.fills.len(), 1);
    assert_eq!(result.fills[0].base_amount, eth_amount(5));
    assert!(result.fully_filled);
}

#[test]
fn test_market_order_sweeps_book() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Market buy more than available at best price
    // Best ask: 10 ETH at $2000, next: 10 ETH at $2010
    let result = pm
        .place_market_order(eth(), usdc(), bob(), OrderSide::Buy, eth_amount(15))
        .unwrap();

    assert_eq!(result.fills.len(), 2);
    assert!(result.fully_filled);

    // First fill at $2000
    assert_eq!(result.fills[0].base_amount, eth_amount(10));
    // Second fill at $2010
    assert_eq!(result.fills[1].base_amount, eth_amount(5));
}

#[test]
fn test_direct_quote_sell() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Get quote for selling 5 ETH for USDC
    let quote = pm.get_quote(eth(), usdc(), eth_amount(5)).unwrap();

    assert_eq!(quote.token_in, eth());
    assert_eq!(quote.token_out, usdc());
    assert_eq!(quote.amount_in, eth_amount(5));
    assert_eq!(quote.route.hops.len(), 1); // Direct route

    // Should get approximately 5 * $1990 = $9,950 USDC (minus fees)
    // The exact amount depends on fee calculation
    assert!(quote.amount_out > U256::ZERO);
}

#[test]
fn test_direct_quote_buy() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Get quote for buying ETH with 10,000 USDC
    let quote = pm.get_quote(usdc(), eth(), usdc_amount(10000)).unwrap();

    assert_eq!(quote.token_in, usdc());
    assert_eq!(quote.token_out, eth());
    assert_eq!(quote.route.hops.len(), 1);
    assert!(quote.amount_out > U256::ZERO);
}

#[test]
fn test_multi_hop_routing() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);
    add_wbtc_usdc_liquidity(&mut pm);

    // No direct LINK/USDC pair, but we have LINK/ETH and ETH/USDC
    // First add some LINK/ETH liquidity
    pm.place_limit_order(
        link(),
        eth(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(10u128.pow(18), 100 * 10u128.pow(18)), // 0.01 ETH per LINK
        U256::from(10000) * U256::from(10u64.pow(18)), // 10000 LINK
    )
    .unwrap();

    pm.place_limit_order(
        link(),
        eth(),
        alice(),
        OrderSide::Buy,
        Price::from_u128(9 * 10u128.pow(15), 10u128.pow(18)), // 0.009 ETH per LINK
        U256::from(10000) * U256::from(10u64.pow(18)),
    )
    .unwrap();

    // Try to get a quote from LINK to USDC (should route through ETH)
    let link_amount = U256::from(100) * U256::from(10u64.pow(18)); // 100 LINK
    let quote = pm.get_quote(link(), usdc(), link_amount).unwrap();

    assert_eq!(quote.token_in, link());
    assert_eq!(quote.token_out, usdc());
    assert_eq!(quote.route.hops.len(), 2); // LINK -> ETH -> USDC
    assert!(quote.amount_out > U256::ZERO);

    // Verify the route
    assert_eq!(quote.route.hops[0].token_in, link());
    assert_eq!(quote.route.hops[0].token_out, eth());
    assert_eq!(quote.route.hops[1].token_in, eth());
    assert_eq!(quote.route.hops[1].token_out, usdc());
}

#[test]
fn test_execute_swap_direct() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Execute a swap: sell 5 ETH for USDC
    let result = pm
        .execute_swap(
            bob(),
            eth(),
            usdc(),
            eth_amount(5),
            U256::ZERO, // No minimum (for testing)
        )
        .unwrap();

    assert_eq!(result.amount_in, eth_amount(5));
    assert!(result.amount_out > U256::ZERO);
    assert_eq!(result.route.hops.len(), 1);
    assert_eq!(result.trades.len(), 1);
}

#[test]
fn test_execute_swap_with_slippage_protection() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Try to execute a swap with unrealistic minimum output
    let result = pm.execute_swap(
        bob(),
        eth(),
        usdc(),
        eth_amount(5),
        usdc_amount(100000), // Want at least $100,000 for 5 ETH - impossible
    );

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        dex::pool_manager::PoolError::SlippageExceeded
    );
}

#[test]
fn test_orderbook_depth() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    let pair = Pair::new(eth(), usdc());
    let orderbook = pm.get_orderbook(&pair).unwrap();

    // Check bid depth
    let bids = orderbook.bid_liquidity(5);
    assert_eq!(bids.len(), 5);

    // Check ask depth
    let asks = orderbook.ask_liquidity(5);
    assert_eq!(asks.len(), 5);

    // Verify best bid < best ask (spread exists)
    let best_bid = orderbook.best_bid().unwrap();
    let best_ask = orderbook.best_ask().unwrap();
    assert!(best_bid < best_ask);
}

#[test]
fn test_cancel_order() {
    let mut pm = setup_market();

    // Place an order
    let (order_id, _) = pm
        .place_limit_order(
            eth(),
            usdc(),
            bob(),
            OrderSide::Buy,
            Price::from_u128(1800 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(10),
        )
        .unwrap();

    // Verify it exists
    let pair = Pair::new(eth(), usdc());
    let orderbook = pm.get_orderbook(&pair).unwrap();
    assert!(orderbook.get_order(order_id).is_some());

    // Cancel it
    pm.cancel_order(eth(), usdc(), order_id).unwrap();

    // Verify it's gone
    let orderbook = pm.get_orderbook(&pair).unwrap();
    assert!(orderbook.get_order(order_id).is_none());
}

#[test]
fn test_multiple_traders() {
    let mut pm = setup_market();

    // Alice places sell orders
    pm.place_limit_order(
        eth(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
        eth_amount(10),
    )
    .unwrap();

    // Bob places sell orders at a better price
    pm.place_limit_order(
        eth(),
        usdc(),
        bob(),
        OrderSide::Sell,
        Price::from_u128(1990 * 10u128.pow(6), 10u128.pow(18)),
        eth_amount(5),
    )
    .unwrap();

    // Charlie buys - should match Bob's order first (better price)
    let (_, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            charlie(),
            OrderSide::Buy,
            Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(3),
        )
        .unwrap();

    assert_eq!(result.fills.len(), 1);
    assert!(result.fully_filled);

    // The fill should be at Bob's price ($1990)
    let fill_price: u128 = result.fills[0].price.numerator.try_into().unwrap();
    assert_eq!(fill_price, 1990 * 10u128.pow(6));
}

#[test]
fn test_fee_calculation() {
    // Create a pool manager with 1% fee
    let config = DexConfig::default().with_fee_bps(100); // 1%
    let mut pm = PoolManager::with_config(config);

    pm.create_pair(eth(), usdc()).unwrap();

    // Add liquidity
    pm.place_limit_order(
        eth(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
        eth_amount(100),
    )
    .unwrap();

    // Get a quote
    let quote = pm.get_quote(usdc(), eth(), usdc_amount(2000)).unwrap();

    // With 1% fee, the fee should be non-zero
    assert!(quote.total_fee > U256::ZERO);
}

#[test]
fn test_pair_stats() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    let stats = pm.pair_stats(eth(), usdc()).unwrap();

    assert!(stats.best_bid.is_some());
    assert!(stats.best_ask.is_some());
    assert!(stats.buy_order_count > 0);
    assert!(stats.sell_order_count > 0);
}

#[test]
fn test_all_stats() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);
    add_wbtc_usdc_liquidity(&mut pm);

    let all_stats = pm.all_stats();

    // Should have stats for all 5 pairs
    assert_eq!(all_stats.len(), 5);
}

#[test]
fn test_no_self_trade_default() {
    let mut pm = setup_market();

    // Alice places a sell order
    pm.place_limit_order(
        eth(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
        eth_amount(10),
    )
    .unwrap();

    // Alice tries to buy her own order - should not match (self-trade prevention)
    let (_, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            alice(),
            OrderSide::Buy,
            Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(5),
        )
        .unwrap();

    // Should not have any fills due to self-trade prevention
    assert!(result.fills.is_empty());
}

#[test]
fn test_self_trade_allowed() {
    let config = DexConfig::default().with_self_trade(true);
    let mut pm = PoolManager::with_config(config);

    pm.create_pair(eth(), usdc()).unwrap();

    // Alice places a sell order
    pm.place_limit_order(
        eth(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
        eth_amount(10),
    )
    .unwrap();

    // Alice tries to buy her own order - should match when self-trade is allowed
    let (_, result) = pm
        .place_limit_order(
            eth(),
            usdc(),
            alice(),
            OrderSide::Buy,
            Price::from_u128(2000 * 10u128.pow(6), 10u128.pow(18)),
            eth_amount(5),
        )
        .unwrap();

    // Should have fills
    assert!(!result.fills.is_empty());
    assert!(result.fully_filled);
}

#[test]
fn test_wbtc_usdc_trading() {
    let mut pm = setup_market();
    add_wbtc_usdc_liquidity(&mut pm);

    // Buy 0.5 BTC
    let result = pm
        .place_market_order(
            wbtc(),
            usdc(),
            bob(),
            OrderSide::Buy,
            btc_amount(1) / U256::from(2), // 0.5 BTC
        )
        .unwrap();

    assert!(!result.fills.is_empty());
    assert!(result.fully_filled);
}

#[test]
fn test_stablecoin_pair() {
    let mut pm = setup_market();

    // Add DAI/USDC liquidity (should be near 1:1)
    pm.place_limit_order(
        dai(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(1001000, 10u128.pow(18)), // 1.001 USDC per DAI
        U256::from(100000) * U256::from(10u64.pow(18)), // 100k DAI
    )
    .unwrap();

    pm.place_limit_order(
        dai(),
        usdc(),
        alice(),
        OrderSide::Buy,
        Price::from_u128(999000, 10u128.pow(18)), // 0.999 USDC per DAI
        U256::from(100000) * U256::from(10u64.pow(18)),
    )
    .unwrap();

    // Swap 1000 DAI for USDC
    let dai_amount = U256::from(1000) * U256::from(10u64.pow(18));
    let quote = pm.get_quote(dai(), usdc(), dai_amount).unwrap();

    // Should get close to 1000 USDC (minus small fee)
    let expected_min = usdc_amount(990); // At least $990
    assert!(quote.amount_out > expected_min);
}

#[test]
fn test_insufficient_liquidity() {
    let mut pm = setup_market();
    add_eth_usdc_liquidity(&mut pm);

    // Try to buy way more ETH than available
    // We only have 40 ETH total in sell orders
    let result = pm.place_market_order(
        eth(),
        usdc(),
        bob(),
        OrderSide::Buy,
        eth_amount(1000), // Way more than available
    );

    // Market order should partially fill
    let result = result.unwrap();
    assert!(!result.fully_filled);
    assert!(result.remaining_amount > U256::ZERO);
}

#[test]
fn test_empty_orderbook_quote() {
    let pm = setup_market();
    // Don't add any liquidity

    // Try to get a quote on empty orderbook
    let result = pm.get_quote(eth(), usdc(), eth_amount(1));

    // Should fail - no liquidity
    assert!(result.is_err());
}

#[test]
fn test_min_order_size() {
    let config = DexConfig::default().with_min_order_size(1000);
    let mut pm = PoolManager::with_config(config);

    pm.create_pair(eth(), usdc()).unwrap();

    // Try to place an order below minimum size
    let result = pm.place_limit_order(
        eth(),
        usdc(),
        alice(),
        OrderSide::Sell,
        Price::from_u128(2000, 1),
        U256::from(500), // Below minimum
    );

    assert!(result.is_err());
}
