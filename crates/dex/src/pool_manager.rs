//! Pool manager for managing multiple orderbooks.

use crate::config::DexConfig;
use crate::order::{OrderId, OrderSide};
use crate::orderbook::{OrderBook, OrderError, TradeResult};
use crate::pair::{Pair, PairId, PairStats};
use crate::router::{Quote, Route, RouteHop, Router};
use crate::types::{Address, Amount, Price, TokenId, U256};
use std::collections::{HashMap, HashSet};

/// The main DEX pool manager.
/// Manages all trading pairs and provides routing for trades.
pub struct PoolManager {
    /// Configuration for the DEX.
    config: DexConfig,
    /// All orderbooks indexed by pair ID.
    orderbooks: HashMap<PairId, OrderBook>,
    /// Index of tokens to their pairs for routing.
    token_pairs: HashMap<TokenId, HashSet<PairId>>,
    /// Router for finding multi-hop paths.
    router: Router,
}

impl PoolManager {
    /// Create a new pool manager with default configuration.
    pub fn new() -> Self {
        Self::with_config(DexConfig::default())
    }

    /// Create a new pool manager with custom configuration.
    pub fn with_config(config: DexConfig) -> Self {
        Self {
            config,
            orderbooks: HashMap::new(),
            token_pairs: HashMap::new(),
            router: Router::new(),
        }
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &DexConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: DexConfig) {
        self.config = config;
    }

    /// Create a new trading pair.
    /// Returns the pair if created, or an error if it already exists.
    pub fn create_pair(&mut self, base: TokenId, quote: TokenId) -> Result<Pair, PoolError> {
        if base == quote {
            return Err(PoolError::InvalidPair);
        }

        let pair = Pair::new(base, quote);
        let pair_id = pair.id();

        if self.orderbooks.contains_key(&pair_id) {
            return Err(PoolError::PairAlreadyExists);
        }

        // Create the orderbook
        self.orderbooks.insert(pair_id, OrderBook::new(pair));

        // Update token index
        self.token_pairs
            .entry(base)
            .or_insert_with(HashSet::new)
            .insert(pair_id);
        self.token_pairs
            .entry(quote)
            .or_insert_with(HashSet::new)
            .insert(pair_id);

        // Update router
        self.router.add_pair(pair);

        Ok(pair)
    }

    /// Get an orderbook by pair.
    pub fn get_orderbook(&self, pair: &Pair) -> Option<&OrderBook> {
        self.orderbooks.get(&pair.id())
    }

    /// Get a mutable orderbook by pair.
    pub fn get_orderbook_mut(&mut self, pair: &Pair) -> Option<&mut OrderBook> {
        self.orderbooks.get_mut(&pair.id())
    }

    /// Get an orderbook by pair ID.
    pub fn get_orderbook_by_id(&self, pair_id: &PairId) -> Option<&OrderBook> {
        self.orderbooks.get(pair_id)
    }

    /// Check if a pair exists.
    pub fn pair_exists(&self, base: TokenId, quote: TokenId) -> bool {
        let pair_id = PairId::from_tokens(base, quote);
        self.orderbooks.contains_key(&pair_id)
    }

    /// Get all active pairs.
    pub fn pairs(&self) -> Vec<Pair> {
        self.orderbooks.values().map(|ob| ob.pair).collect()
    }

    /// Get all pairs containing a specific token.
    pub fn pairs_for_token(&self, token: TokenId) -> Vec<Pair> {
        self.token_pairs
            .get(&token)
            .map(|pair_ids| {
                pair_ids
                    .iter()
                    .filter_map(|id| self.orderbooks.get(id).map(|ob| ob.pair))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Place a limit order on a pair.
    pub fn place_limit_order(
        &mut self,
        base: TokenId,
        quote: TokenId,
        trader: Address,
        side: OrderSide,
        price: Price,
        amount: Amount,
    ) -> Result<(OrderId, TradeResult), PoolError> {
        let pair = Pair::new(base, quote);
        let orderbook = self
            .orderbooks
            .get_mut(&pair.id())
            .ok_or(PoolError::PairNotFound)?;

        orderbook
            .place_limit_order(trader, side, price, amount, &self.config)
            .map_err(PoolError::OrderError)
    }

    /// Place a market order on a pair.
    pub fn place_market_order(
        &mut self,
        base: TokenId,
        quote: TokenId,
        trader: Address,
        side: OrderSide,
        amount: Amount,
    ) -> Result<TradeResult, PoolError> {
        let pair = Pair::new(base, quote);
        let orderbook = self
            .orderbooks
            .get_mut(&pair.id())
            .ok_or(PoolError::PairNotFound)?;

        orderbook
            .place_market_order(trader, side, amount, &self.config)
            .map_err(PoolError::OrderError)
    }

    /// Cancel an order.
    pub fn cancel_order(
        &mut self,
        base: TokenId,
        quote: TokenId,
        order_id: OrderId,
    ) -> Result<(), PoolError> {
        let pair = Pair::new(base, quote);
        let orderbook = self
            .orderbooks
            .get_mut(&pair.id())
            .ok_or(PoolError::PairNotFound)?;

        orderbook
            .cancel_order(order_id)
            .map_err(PoolError::OrderError)?;
        Ok(())
    }

    /// Get a quote for swapping tokens.
    /// This will find the best route (direct or multi-hop) and return the expected output.
    pub fn get_quote(
        &self,
        token_in: TokenId,
        token_out: TokenId,
        amount_in: Amount,
    ) -> Result<Quote, PoolError> {
        if token_in == token_out {
            return Err(PoolError::InvalidPair);
        }

        if amount_in.is_zero() {
            return Err(PoolError::InvalidAmount);
        }

        // Try direct route first
        if let Some(quote) = self.get_direct_quote(token_in, token_out, amount_in) {
            return Ok(quote);
        }

        // Try multi-hop routing
        self.get_routed_quote(token_in, token_out, amount_in)
    }

    /// Get a quote for a direct pair (no routing).
    fn get_direct_quote(
        &self,
        token_in: TokenId,
        token_out: TokenId,
        amount_in: Amount,
    ) -> Option<Quote> {
        // Check if pair exists in either direction
        let pair_id = PairId::from_tokens(token_in, token_out);
        let orderbook = self.orderbooks.get(&pair_id)?;

        // Determine if we're buying or selling
        let (amount_out, _avg_price) = if orderbook.pair.base == token_in {
            // We have base, want quote -> sell
            orderbook.simulate_market_sell(amount_in, &self.config)?
        } else {
            // We have quote, want base -> buy
            orderbook.simulate_market_buy(amount_in, &self.config)?
        };

        Some(Quote {
            token_in,
            token_out,
            amount_in,
            amount_out,
            route: Route {
                hops: vec![RouteHop {
                    pair: orderbook.pair,
                    token_in,
                    token_out,
                }],
            },
            price_impact: self.calculate_price_impact(&orderbook, token_in, amount_in),
            total_fee: U256::from(self.config.calculate_fee(amount_out.try_into().unwrap_or(u128::MAX))),
        })
    }

    /// Get a quote using multi-hop routing.
    fn get_routed_quote(
        &self,
        token_in: TokenId,
        token_out: TokenId,
        amount_in: Amount,
    ) -> Result<Quote, PoolError> {
        // Find possible routes
        let routes = self.router.find_routes(
            token_in,
            token_out,
            self.config.max_routing_hops,
            &self.orderbooks,
        );

        if routes.is_empty() {
            return Err(PoolError::NoRouteFound);
        }

        // Evaluate each route and find the best one
        let mut best_quote: Option<Quote> = None;

        for route in routes {
            if let Some(quote) = self.evaluate_route(&route, amount_in) {
                match &best_quote {
                    None => best_quote = Some(quote),
                    Some(current_best) if quote.amount_out > current_best.amount_out => {
                        best_quote = Some(quote);
                    }
                    _ => {}
                }
            }
        }

        best_quote.ok_or(PoolError::InsufficientLiquidity)
    }

    /// Evaluate a route to get the expected output.
    fn evaluate_route(&self, route: &Route, amount_in: Amount) -> Option<Quote> {
        let mut current_amount = amount_in;
        let mut total_fee = U256::ZERO;

        for hop in &route.hops {
            let pair_id = hop.pair.id();
            let orderbook = self.orderbooks.get(&pair_id)?;

            let (amount_out, _) = if orderbook.pair.base == hop.token_in {
                // Selling base for quote
                orderbook.simulate_market_sell(current_amount, &self.config)?
            } else {
                // Buying base with quote
                orderbook.simulate_market_buy(current_amount, &self.config)?
            };

            let hop_fee = U256::from(self.config.calculate_fee(amount_out.try_into().unwrap_or(u128::MAX)));
            total_fee = total_fee.saturating_add(hop_fee);
            current_amount = amount_out;
        }

        let first_hop = route.hops.first()?;
        let last_hop = route.hops.last()?;

        Some(Quote {
            token_in: first_hop.token_in,
            token_out: last_hop.token_out,
            amount_in,
            amount_out: current_amount,
            route: route.clone(),
            price_impact: U256::ZERO, // TODO: Calculate cumulative price impact
            total_fee,
        })
    }

    /// Calculate price impact for a trade.
    fn calculate_price_impact(
        &self,
        orderbook: &OrderBook,
        token_in: TokenId,
        amount_in: Amount,
    ) -> U256 {
        // Get mid price
        let (best_bid, best_ask) = match orderbook.spread() {
            Some(spread) => spread,
            None => return U256::ZERO,
        };

        // Calculate mid price (average of bid and ask)
        let mid_numerator = best_bid.numerator.saturating_add(best_ask.numerator);
        let mid_denominator = best_bid.denominator.saturating_add(best_ask.denominator);

        if mid_denominator.is_zero() {
            return U256::ZERO;
        }

        // Simulate the trade to get execution price
        let (_amount_out, exec_price) = if orderbook.pair.base == token_in {
            match orderbook.simulate_market_sell(amount_in, &self.config) {
                Some(r) => r,
                None => return U256::ZERO,
            }
        } else {
            match orderbook.simulate_market_buy(amount_in, &self.config) {
                Some(r) => r,
                None => return U256::ZERO,
            }
        };

        // Price impact = |exec_price - mid_price| / mid_price * 10000 (in basis points)
        // Simplified calculation
        let impact_bps = exec_price
            .numerator
            .saturating_mul(U256::from(10000))
            .checked_div(mid_numerator)
            .unwrap_or(U256::ZERO);

        impact_bps.saturating_sub(U256::from(10000))
    }

    /// Execute a swap along a route.
    /// This actually executes the trades (not just a simulation).
    pub fn execute_swap(
        &mut self,
        trader: Address,
        token_in: TokenId,
        token_out: TokenId,
        amount_in: Amount,
        min_amount_out: Amount,
    ) -> Result<SwapResult, PoolError> {
        // Get the best quote first
        let quote = self.get_quote(token_in, token_out, amount_in)?;

        if quote.amount_out < min_amount_out {
            return Err(PoolError::SlippageExceeded);
        }

        // Execute each hop
        let mut current_amount = amount_in;
        let mut all_trades = Vec::new();

        for hop in &quote.route.hops {
            let pair_id = hop.pair.id();
            let orderbook = self
                .orderbooks
                .get_mut(&pair_id)
                .ok_or(PoolError::PairNotFound)?;

            let side = if orderbook.pair.base == hop.token_in {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            };

            let trade_result = orderbook
                .place_market_order(trader, side, current_amount, &self.config)
                .map_err(PoolError::OrderError)?;

            // Calculate output from fills
            let output: Amount = trade_result
                .fills
                .iter()
                .map(|f| {
                    if side == OrderSide::Sell {
                        f.quote_amount
                    } else {
                        f.base_amount
                    }
                })
                .fold(U256::ZERO, |acc, x| acc.saturating_add(x));

            current_amount = output;
            all_trades.push(trade_result);
        }

        Ok(SwapResult {
            amount_in,
            amount_out: current_amount,
            route: quote.route,
            trades: all_trades,
        })
    }

    /// Get statistics for all pairs.
    pub fn all_stats(&self) -> HashMap<Pair, PairStats> {
        self.orderbooks
            .values()
            .map(|ob| (ob.pair, ob.stats()))
            .collect()
    }

    /// Get statistics for a specific pair.
    pub fn pair_stats(&self, base: TokenId, quote: TokenId) -> Option<PairStats> {
        let pair = Pair::new(base, quote);
        self.orderbooks.get(&pair.id()).map(|ob| ob.stats())
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing a swap.
#[derive(Debug)]
pub struct SwapResult {
    /// Amount of input token.
    pub amount_in: Amount,
    /// Amount of output token received.
    pub amount_out: Amount,
    /// The route taken.
    pub route: Route,
    /// Trade results for each hop.
    pub trades: Vec<TradeResult>,
}

/// Errors that can occur in the pool manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    /// The trading pair already exists.
    PairAlreadyExists,
    /// The trading pair was not found.
    PairNotFound,
    /// Invalid pair (e.g., same token on both sides).
    InvalidPair,
    /// Invalid amount.
    InvalidAmount,
    /// No route found between tokens.
    NoRouteFound,
    /// Insufficient liquidity for the trade.
    InsufficientLiquidity,
    /// Slippage tolerance exceeded.
    SlippageExceeded,
    /// Order-related error.
    OrderError(OrderError),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PairAlreadyExists => write!(f, "pair already exists"),
            PoolError::PairNotFound => write!(f, "pair not found"),
            PoolError::InvalidPair => write!(f, "invalid pair"),
            PoolError::InvalidAmount => write!(f, "invalid amount"),
            PoolError::NoRouteFound => write!(f, "no route found"),
            PoolError::InsufficientLiquidity => write!(f, "insufficient liquidity"),
            PoolError::SlippageExceeded => write!(f, "slippage tolerance exceeded"),
            PoolError::OrderError(e) => write!(f, "order error: {}", e),
        }
    }
}

impl std::error::Error for PoolError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ETH_TOKEN;

    fn setup_tokens() -> (TokenId, TokenId, TokenId) {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);
        let wbtc = Address::repeat_byte(0x02);
        (eth, usdc, wbtc)
    }

    fn test_trader(n: u8) -> Address {
        Address::repeat_byte(n + 0x10)
    }

    #[test]
    fn test_create_pair() {
        let mut pm = PoolManager::new();
        let (eth, usdc, _) = setup_tokens();

        let pair = pm.create_pair(eth, usdc).unwrap();
        assert_eq!(pair.base, eth);
        assert_eq!(pair.quote, usdc);

        // Can't create duplicate
        assert!(matches!(
            pm.create_pair(eth, usdc),
            Err(PoolError::PairAlreadyExists)
        ));

        // Can't create with same token
        assert!(matches!(pm.create_pair(eth, eth), Err(PoolError::InvalidPair)));
    }

    #[test]
    fn test_place_orders() {
        let mut pm = PoolManager::new();
        let (eth, usdc, _) = setup_tokens();
        let trader = test_trader(1);

        pm.create_pair(eth, usdc).unwrap();

        // Place a limit order
        let (_order_id, result) = pm
            .place_limit_order(
                eth,
                usdc,
                trader,
                OrderSide::Sell,
                Price::from_u128(2000, 1),
                U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            )
            .unwrap();

        assert!(result.fills.is_empty());
    }

    #[test]
    fn test_direct_quote() {
        let mut pm = PoolManager::new();
        let (eth, usdc, _) = setup_tokens();
        let maker = test_trader(1);

        pm.create_pair(eth, usdc).unwrap();

        // Add liquidity: sell 10 ETH at 2000 USDC each
        pm.place_limit_order(
            eth,
            usdc,
            maker,
            OrderSide::Sell,
            Price::from_u128(2000, 1),
            U256::from(10_000_000_000_000_000_000u128), // 10 ETH
        )
        .unwrap();

        // Get quote for buying ETH with USDC
        let quote = pm
            .get_quote(usdc, eth, U256::from(2000_000_000u128)) // 2000 USDC (assuming 6 decimals)
            .unwrap();

        assert_eq!(quote.token_in, usdc);
        assert_eq!(quote.token_out, eth);
        assert_eq!(quote.route.hops.len(), 1);
    }
}
