//! Multi-hop routing for finding the best path between tokens.

use crate::orderbook::OrderBook;
use crate::pair::{Pair, PairId};
use crate::types::{Amount, TokenId, U256};
use std::collections::{HashMap, HashSet, VecDeque};

/// A single hop in a route.
#[derive(Debug, Clone)]
pub struct RouteHop {
    /// The trading pair for this hop.
    pub pair: Pair,
    /// The input token for this hop.
    pub token_in: TokenId,
    /// The output token for this hop.
    pub token_out: TokenId,
}

/// A complete route from one token to another.
#[derive(Debug, Clone)]
pub struct Route {
    /// The hops in this route.
    pub hops: Vec<RouteHop>,
}

impl Route {
    /// Get the number of hops in this route.
    pub fn len(&self) -> usize {
        self.hops.len()
    }

    /// Check if the route is empty.
    pub fn is_empty(&self) -> bool {
        self.hops.is_empty()
    }

    /// Get the input token.
    pub fn token_in(&self) -> Option<TokenId> {
        self.hops.first().map(|h| h.token_in)
    }

    /// Get the output token.
    pub fn token_out(&self) -> Option<TokenId> {
        self.hops.last().map(|h| h.token_out)
    }
}

/// A quote for a swap.
#[derive(Debug, Clone)]
pub struct Quote {
    /// The input token.
    pub token_in: TokenId,
    /// The output token.
    pub token_out: TokenId,
    /// The input amount.
    pub amount_in: Amount,
    /// The expected output amount (after fees).
    pub amount_out: Amount,
    /// The route to execute.
    pub route: Route,
    /// Estimated price impact in basis points.
    pub price_impact: U256,
    /// Total fees paid.
    pub total_fee: Amount,
}

/// Router for finding paths between tokens.
#[derive(Debug)]
pub struct Router {
    /// Graph of token connections.
    /// Maps each token to a set of tokens it can be traded with directly.
    graph: HashMap<TokenId, HashSet<TokenId>>,
    /// Maps token pairs to their pair info.
    pairs: HashMap<(TokenId, TokenId), Pair>,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            pairs: HashMap::new(),
        }
    }

    /// Add a trading pair to the router.
    pub fn add_pair(&mut self, pair: Pair) {
        // Add bidirectional edges
        self.graph
            .entry(pair.base)
            .or_insert_with(HashSet::new)
            .insert(pair.quote);
        self.graph
            .entry(pair.quote)
            .or_insert_with(HashSet::new)
            .insert(pair.base);

        // Store pair info for both directions
        self.pairs.insert((pair.base, pair.quote), pair);
        self.pairs.insert((pair.quote, pair.base), pair);
    }

    /// Remove a trading pair from the router.
    pub fn remove_pair(&mut self, pair: Pair) {
        if let Some(neighbors) = self.graph.get_mut(&pair.base) {
            neighbors.remove(&pair.quote);
        }
        if let Some(neighbors) = self.graph.get_mut(&pair.quote) {
            neighbors.remove(&pair.base);
        }
        self.pairs.remove(&(pair.base, pair.quote));
        self.pairs.remove(&(pair.quote, pair.base));
    }

    /// Find all routes between two tokens up to a maximum number of hops.
    /// Uses BFS to find shortest paths first.
    pub fn find_routes(
        &self,
        token_in: TokenId,
        token_out: TokenId,
        max_hops: usize,
        orderbooks: &HashMap<PairId, OrderBook>,
    ) -> Vec<Route> {
        let mut routes = Vec::new();

        // BFS to find all paths
        let mut queue: VecDeque<(TokenId, Vec<TokenId>)> = VecDeque::new();
        queue.push_back((token_in, vec![token_in]));

        while let Some((current, path)) = queue.pop_front() {
            // Skip if we've exceeded max hops
            if path.len() > max_hops + 1 {
                continue;
            }

            // Check if we've reached the destination
            if current == token_out && path.len() > 1 {
                if let Some(route) = self.path_to_route(&path, orderbooks) {
                    routes.push(route);
                }
                continue;
            }

            // Explore neighbors
            if let Some(neighbors) = self.graph.get(&current) {
                for &neighbor in neighbors {
                    // Avoid cycles
                    if path.contains(&neighbor) {
                        continue;
                    }

                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push_back((neighbor, new_path));
                }
            }
        }

        // Sort routes by number of hops (prefer shorter routes)
        routes.sort_by_key(|r| r.len());

        routes
    }

    /// Convert a path of tokens to a Route.
    fn path_to_route(
        &self,
        path: &[TokenId],
        orderbooks: &HashMap<PairId, OrderBook>,
    ) -> Option<Route> {
        if path.len() < 2 {
            return None;
        }

        let mut hops = Vec::with_capacity(path.len() - 1);

        for window in path.windows(2) {
            let token_in = window[0];
            let token_out = window[1];

            // Get the pair
            let pair = self.pairs.get(&(token_in, token_out))?;

            // Verify the orderbook exists and has liquidity
            let pair_id = pair.id();
            let _orderbook = orderbooks.get(&pair_id)?;

            hops.push(RouteHop {
                pair: *pair,
                token_in,
                token_out,
            });
        }

        Some(Route { hops })
    }

    /// Find the best route between two tokens based on expected output.
    /// This requires simulating each route to find the one with the highest output.
    pub fn find_best_route(
        &self,
        token_in: TokenId,
        token_out: TokenId,
        amount_in: Amount,
        max_hops: usize,
        orderbooks: &HashMap<PairId, OrderBook>,
        config: &crate::config::DexConfig,
    ) -> Option<(Route, Amount)> {
        let routes = self.find_routes(token_in, token_out, max_hops, orderbooks);

        let mut best: Option<(Route, Amount)> = None;

        for route in routes {
            if let Some(output) = self.simulate_route(&route, amount_in, orderbooks, config) {
                match &best {
                    None => best = Some((route, output)),
                    Some((_, current_best)) if output > *current_best => {
                        best = Some((route, output));
                    }
                    _ => {}
                }
            }
        }

        best
    }

    /// Simulate a route to get the expected output.
    fn simulate_route(
        &self,
        route: &Route,
        amount_in: Amount,
        orderbooks: &HashMap<PairId, OrderBook>,
        config: &crate::config::DexConfig,
    ) -> Option<Amount> {
        let mut current_amount = amount_in;

        for hop in &route.hops {
            let pair_id = hop.pair.id();
            let orderbook = orderbooks.get(&pair_id)?;

            // Determine direction
            let (output, _) = if orderbook.pair.base == hop.token_in {
                // Selling base for quote
                orderbook.simulate_market_sell(current_amount, config)?
            } else {
                // Buying base with quote
                orderbook.simulate_market_buy(current_amount, config)?
            };

            current_amount = output;
        }

        Some(current_amount)
    }

    /// Get all tokens that are tradeable.
    pub fn all_tokens(&self) -> Vec<TokenId> {
        self.graph.keys().copied().collect()
    }

    /// Get all tokens that can be reached from a given token.
    pub fn reachable_tokens(&self, from: TokenId) -> HashSet<TokenId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(neighbors) = self.graph.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }

        visited.remove(&from);
        visited
    }

    /// Check if there's any path between two tokens.
    pub fn has_path(&self, from: TokenId, to: TokenId) -> bool {
        if from == to {
            return true;
        }
        self.reachable_tokens(from).contains(&to)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ETH_TOKEN;

    fn setup_tokens() -> (TokenId, TokenId, TokenId, TokenId) {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);
        let wbtc = Address::repeat_byte(0x02);
        let dai = Address::repeat_byte(0x03);
        (eth, usdc, wbtc, dai)
    }

    use crate::types::Address;

    #[test]
    fn test_router_add_pair() {
        let mut router = Router::new();
        let (eth, usdc, _, _) = setup_tokens();

        let pair = Pair::new(eth, usdc);
        router.add_pair(pair);

        assert!(router.graph.get(&eth).unwrap().contains(&usdc));
        assert!(router.graph.get(&usdc).unwrap().contains(&eth));
    }

    #[test]
    fn test_router_has_path() {
        let mut router = Router::new();
        let (eth, usdc, wbtc, dai) = setup_tokens();

        // ETH <-> USDC <-> WBTC
        router.add_pair(Pair::new(eth, usdc));
        router.add_pair(Pair::new(usdc, wbtc));

        assert!(router.has_path(eth, usdc)); // Direct
        assert!(router.has_path(eth, wbtc)); // Via USDC
        assert!(!router.has_path(eth, dai)); // No connection
    }

    #[test]
    fn test_find_routes() {
        let mut router = Router::new();
        let (eth, usdc, wbtc, _) = setup_tokens();

        // Create a triangle: ETH <-> USDC <-> WBTC <-> ETH
        let pair1 = Pair::new(eth, usdc);
        let pair2 = Pair::new(usdc, wbtc);
        let pair3 = Pair::new(eth, wbtc);

        router.add_pair(pair1);
        router.add_pair(pair2);
        router.add_pair(pair3);

        // Create mock orderbooks
        let mut orderbooks = HashMap::new();
        orderbooks.insert(pair1.id(), OrderBook::new(pair1));
        orderbooks.insert(pair2.id(), OrderBook::new(pair2));
        orderbooks.insert(pair3.id(), OrderBook::new(pair3));

        let routes = router.find_routes(eth, wbtc, 2, &orderbooks);

        // Should find direct route and route via USDC
        assert!(routes.len() >= 2);

        // First route should be direct (shorter)
        assert_eq!(routes[0].len(), 1);
    }

    #[test]
    fn test_reachable_tokens() {
        let mut router = Router::new();
        let (eth, usdc, wbtc, dai) = setup_tokens();

        router.add_pair(Pair::new(eth, usdc));
        router.add_pair(Pair::new(usdc, wbtc));
        // DAI is not connected

        let reachable = router.reachable_tokens(eth);
        assert!(reachable.contains(&usdc));
        assert!(reachable.contains(&wbtc));
        assert!(!reachable.contains(&dai));
    }
}
