//! Orderbook implementation with efficient order matching.

use crate::config::DexConfig;
use crate::order::{Order, OrderId, OrderSide};
use crate::pair::{Pair, PairStats};
use crate::types::{Address, Amount, Price, U256};
use std::collections::{BTreeMap, HashMap};

/// Result of executing a trade.
#[derive(Debug, Clone)]
pub struct TradeResult {
    /// The taker order ID.
    pub taker_order_id: OrderId,
    /// List of fills that occurred.
    pub fills: Vec<Fill>,
    /// Remaining unfilled amount of the taker order.
    pub remaining_amount: Amount,
    /// Whether the order was fully filled.
    pub fully_filled: bool,
}

/// A single fill (partial or complete match between two orders).
#[derive(Debug, Clone)]
pub struct Fill {
    /// The maker order ID.
    pub maker_order_id: OrderId,
    /// The maker's address (for token transfers).
    pub maker: Address,
    /// Amount of base token traded.
    pub base_amount: Amount,
    /// Amount of quote token traded.
    pub quote_amount: Amount,
    /// Price at which the trade occurred.
    pub price: Price,
    /// Fee paid by the taker.
    pub taker_fee: Amount,
    /// Fee paid by the maker.
    pub maker_fee: Amount,
}

/// An orderbook for a single trading pair.
///
/// Uses BTreeMap for price levels to maintain sorted order:
/// - Buy orders (bids): sorted descending by price (highest first)
/// - Sell orders (asks): sorted ascending by price (lowest first)
#[derive(Debug)]
pub struct OrderBook {
    /// The trading pair.
    pub pair: Pair,
    /// Buy orders indexed by price level, each level is a list of orders (FIFO).
    /// Key is negated for descending order (highest price first).
    bids: BTreeMap<PriceKey, Vec<Order>>,
    /// Sell orders indexed by price level, each level is a list of orders (FIFO).
    asks: BTreeMap<PriceKey, Vec<Order>>,
    /// All orders by ID for quick lookup.
    orders: HashMap<OrderId, OrderLocation>,
    /// Next order ID.
    next_order_id: u64,
    /// Total traded volume.
    total_volume: Amount,
}

/// Location of an order in the book.
#[derive(Debug, Clone)]
struct OrderLocation {
    side: OrderSide,
    price_key: PriceKey,
}

/// Price key for BTreeMap ordering.
/// For bids: we negate to get descending order.
/// For asks: we use normal ordering for ascending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PriceKey {
    /// The actual price.
    price: Price,
    /// Whether this is for a bid (negated for descending).
    is_bid: bool,
}

impl PartialOrd for PriceKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriceKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_bid {
            // For bids, reverse the order (highest first)
            other.price.cmp(&self.price)
        } else {
            // For asks, normal order (lowest first)
            self.price.cmp(&other.price)
        }
    }
}

impl OrderBook {
    /// Create a new orderbook for a pair.
    pub fn new(pair: Pair) -> Self {
        Self {
            pair,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            next_order_id: 1,
            total_volume: U256::ZERO,
        }
    }

    /// Generate a new order ID.
    fn generate_order_id(&mut self) -> OrderId {
        let id = OrderId(self.next_order_id);
        self.next_order_id += 1;
        id
    }

    /// Place a limit order.
    /// Returns the order ID and any immediate fills if the order crosses the spread.
    pub fn place_limit_order(
        &mut self,
        trader: Address,
        side: OrderSide,
        price: Price,
        amount: Amount,
        config: &DexConfig,
    ) -> Result<(OrderId, TradeResult), OrderError> {
        if amount < U256::from(config.min_order_size) {
            return Err(OrderError::BelowMinimumSize);
        }

        let order_id = self.generate_order_id();
        let mut order = Order::new_limit(order_id, trader, side, price, amount);

        // Try to match immediately against existing orders
        let trade_result = self.match_order(&mut order, config);

        // If there's remaining amount, add to the book
        if !order.remaining_amount.is_zero() && order.is_active() {
            self.add_order_to_book(order);
        }

        Ok((order_id, trade_result))
    }

    /// Place a market order.
    /// Market orders execute immediately at the best available price.
    pub fn place_market_order(
        &mut self,
        trader: Address,
        side: OrderSide,
        amount: Amount,
        config: &DexConfig,
    ) -> Result<TradeResult, OrderError> {
        if amount < U256::from(config.min_order_size) {
            return Err(OrderError::BelowMinimumSize);
        }

        let order_id = self.generate_order_id();
        let mut order = Order::new_market(order_id, trader, side, amount);

        // Execute immediately
        let trade_result = self.match_order(&mut order, config);

        // Market orders don't rest on the book - any unfilled portion is cancelled

        Ok(trade_result)
    }

    /// Match an incoming order against the book.
    fn match_order(&mut self, taker_order: &mut Order, config: &DexConfig) -> TradeResult {
        let mut fills = Vec::new();

        // Get the opposite side's orders
        let opposite_book = match taker_order.side {
            OrderSide::Buy => &mut self.asks,
            OrderSide::Sell => &mut self.bids,
        };

        // Collect price levels to remove after matching
        let mut empty_levels: Vec<PriceKey> = Vec::new();

        // Iterate through price levels in order
        for (price_key, orders) in opposite_book.iter_mut() {
            if taker_order.remaining_amount.is_zero() {
                break;
            }

            // Check if prices are compatible
            let can_match = match taker_order.side {
                OrderSide::Buy => taker_order.price >= price_key.price,
                OrderSide::Sell => taker_order.price <= price_key.price,
            };

            if !can_match {
                break; // No more matches possible due to price ordering
            }

            // Match against orders at this price level
            let mut i = 0;
            while i < orders.len() && !taker_order.remaining_amount.is_zero() {
                let maker_order = &mut orders[i];

                if !maker_order.is_active() {
                    i += 1;
                    continue;
                }

                // Check self-trade
                if !config.allow_self_trade && taker_order.trader == maker_order.trader {
                    i += 1;
                    continue;
                }

                // Calculate fill amount
                let fill_base_amount = taker_order
                    .remaining_amount
                    .min(maker_order.remaining_amount);

                // Calculate quote amount using maker's price (price-time priority)
                let fill_quote_amount = maker_order
                    .price
                    .quote_amount(fill_base_amount)
                    .unwrap_or(U256::ZERO);

                // Calculate fees
                let taker_fee = U256::from(
                    config.calculate_fee(fill_quote_amount.try_into().unwrap_or(u128::MAX)),
                );
                let maker_fee = U256::ZERO; // Makers typically don't pay fees

                // Execute the fill
                taker_order.fill(fill_base_amount);
                maker_order.fill(fill_base_amount);

                fills.push(Fill {
                    maker_order_id: maker_order.id,
                    maker: maker_order.trader,
                    base_amount: fill_base_amount,
                    quote_amount: fill_quote_amount,
                    price: maker_order.price,
                    taker_fee,
                    maker_fee,
                });

                // Update volume
                self.total_volume = self.total_volume.saturating_add(fill_base_amount);

                // Remove filled orders from location map
                if !maker_order.is_active() {
                    self.orders.remove(&maker_order.id);
                }

                i += 1;
            }

            // Remove fully filled orders
            orders.retain(|o| o.is_active());

            if orders.is_empty() {
                empty_levels.push(*price_key);
            }
        }

        // Remove empty price levels
        for level in empty_levels {
            match taker_order.side {
                OrderSide::Buy => self.asks.remove(&level),
                OrderSide::Sell => self.bids.remove(&level),
            };
        }

        let remaining = taker_order.remaining_amount;
        let fully_filled = remaining.is_zero();

        TradeResult {
            taker_order_id: taker_order.id,
            fills,
            remaining_amount: remaining,
            fully_filled,
        }
    }

    /// Add an order to the book (after matching).
    fn add_order_to_book(&mut self, order: Order) {
        let price_key = PriceKey {
            price: order.price,
            is_bid: order.side == OrderSide::Buy,
        };

        let book = match order.side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        let orders = book.entry(price_key).or_insert_with(Vec::new);
        orders.push(order.clone());

        self.orders.insert(
            order.id,
            OrderLocation {
                side: order.side,
                price_key,
            },
        );
    }

    /// Cancel an order by ID.
    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<Order, OrderError> {
        let location = self
            .orders
            .remove(&order_id)
            .ok_or(OrderError::OrderNotFound)?;

        let book = match location.side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        let orders = book
            .get_mut(&location.price_key)
            .ok_or(OrderError::OrderNotFound)?;

        // Find and remove the order
        let order_idx = orders
            .iter()
            .position(|o| o.id == order_id)
            .ok_or(OrderError::OrderNotFound)?;

        let mut order = orders.remove(order_idx);
        order.cancel();

        // Clean up empty price levels
        if orders.is_empty() {
            book.remove(&location.price_key);
        }

        Ok(order)
    }

    /// Get an order by ID.
    pub fn get_order(&self, order_id: OrderId) -> Option<&Order> {
        let location = self.orders.get(&order_id)?;
        let book = match location.side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };
        let orders = book.get(&location.price_key)?;
        orders.iter().find(|o| o.id == order_id)
    }

    /// Get the best bid price.
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.first_key_value().map(|(k, _)| k.price)
    }

    /// Get the best ask price.
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first_key_value().map(|(k, _)| k.price)
    }

    /// Get the spread (difference between best ask and best bid).
    pub fn spread(&self) -> Option<(Price, Price)> {
        Some((self.best_bid()?, self.best_ask()?))
    }

    /// Get the total liquidity available at a price level.
    pub fn liquidity_at_price(&self, side: OrderSide, price: Price) -> Amount {
        let price_key = PriceKey {
            price,
            is_bid: side == OrderSide::Buy,
        };

        let book = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };

        book.get(&price_key)
            .map(|orders| {
                orders
                    .iter()
                    .filter(|o| o.is_active())
                    .fold(U256::ZERO, |acc, o| acc.saturating_add(o.remaining_amount))
            })
            .unwrap_or(U256::ZERO)
    }

    /// Get total bid liquidity up to a certain depth.
    pub fn bid_liquidity(&self, depth: usize) -> Vec<(Price, Amount)> {
        self.bids
            .iter()
            .take(depth)
            .map(|(k, orders)| {
                let total = orders
                    .iter()
                    .filter(|o| o.is_active())
                    .fold(U256::ZERO, |acc, o| acc.saturating_add(o.remaining_amount));
                (k.price, total)
            })
            .collect()
    }

    /// Get total ask liquidity up to a certain depth.
    pub fn ask_liquidity(&self, depth: usize) -> Vec<(Price, Amount)> {
        self.asks
            .iter()
            .take(depth)
            .map(|(k, orders)| {
                let total = orders
                    .iter()
                    .filter(|o| o.is_active())
                    .fold(U256::ZERO, |acc, o| acc.saturating_add(o.remaining_amount));
                (k.price, total)
            })
            .collect()
    }

    /// Get statistics about the orderbook.
    pub fn stats(&self) -> PairStats {
        let best_bid = self.best_bid().map(|p| p.numerator);
        let best_ask = self.best_ask().map(|p| p.numerator);

        let buy_order_count = self.bids.values().map(|v| v.len()).sum();
        let sell_order_count = self.asks.values().map(|v| v.len()).sum();

        PairStats {
            best_bid,
            best_ask,
            total_volume: self.total_volume,
            buy_order_count,
            sell_order_count,
        }
    }

    /// Simulate a market buy to get expected output.
    /// Returns (output_amount, average_price) if there's enough liquidity.
    pub fn simulate_market_buy(
        &self,
        input_quote_amount: Amount,
        config: &DexConfig,
    ) -> Option<(Amount, Price)> {
        let mut remaining_quote = input_quote_amount;
        let mut total_base = U256::ZERO;

        for (_price_key, orders) in &self.asks {
            if remaining_quote.is_zero() {
                break;
            }

            for order in orders {
                if !order.is_active() || remaining_quote.is_zero() {
                    continue;
                }

                // How much quote do we need to buy all of this order's base?
                let order_quote_value = order.price.quote_amount(order.remaining_amount)?;

                if remaining_quote >= order_quote_value {
                    // We can fill this entire order
                    total_base = total_base.saturating_add(order.remaining_amount);
                    remaining_quote = remaining_quote.saturating_sub(order_quote_value);
                } else {
                    // Partial fill
                    let base_we_can_buy = order.price.base_amount(remaining_quote)?;
                    total_base = total_base.saturating_add(base_we_can_buy);
                    remaining_quote = U256::ZERO;
                }
            }
        }

        if total_base.is_zero() {
            return None;
        }

        // Apply fee to output
        let fee = U256::from(config.calculate_fee(total_base.try_into().unwrap_or(u128::MAX)));
        let output_after_fee = total_base.saturating_sub(fee);

        // Calculate average price
        let spent = input_quote_amount.saturating_sub(remaining_quote);
        let avg_price = Price::new(spent, total_base);

        Some((output_after_fee, avg_price))
    }

    /// Simulate a market sell to get expected output.
    /// Returns (output_amount, average_price) if there's enough liquidity.
    pub fn simulate_market_sell(
        &self,
        input_base_amount: Amount,
        config: &DexConfig,
    ) -> Option<(Amount, Price)> {
        let mut remaining_base = input_base_amount;
        let mut total_quote = U256::ZERO;

        for (_price_key, orders) in &self.bids {
            if remaining_base.is_zero() {
                break;
            }

            for order in orders {
                if !order.is_active() || remaining_base.is_zero() {
                    continue;
                }

                if remaining_base >= order.remaining_amount {
                    // We can fill this entire order
                    let quote_value = order.price.quote_amount(order.remaining_amount)?;
                    total_quote = total_quote.saturating_add(quote_value);
                    remaining_base = remaining_base.saturating_sub(order.remaining_amount);
                } else {
                    // Partial fill
                    let quote_value = order.price.quote_amount(remaining_base)?;
                    total_quote = total_quote.saturating_add(quote_value);
                    remaining_base = U256::ZERO;
                }
            }
        }

        if total_quote.is_zero() {
            return None;
        }

        // Apply fee to output
        let fee = U256::from(config.calculate_fee(total_quote.try_into().unwrap_or(u128::MAX)));
        let output_after_fee = total_quote.saturating_sub(fee);

        // Calculate average price
        let sold = input_base_amount.saturating_sub(remaining_base);
        let avg_price = Price::new(total_quote, sold);

        Some((output_after_fee, avg_price))
    }
}

/// Errors that can occur when working with orders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    /// Order amount is below the minimum size.
    BelowMinimumSize,
    /// Order was not found.
    OrderNotFound,
    /// Insufficient liquidity for the trade.
    InsufficientLiquidity,
    /// Invalid price.
    InvalidPrice,
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderError::BelowMinimumSize => write!(f, "order amount is below minimum size"),
            OrderError::OrderNotFound => write!(f, "order not found"),
            OrderError::InsufficientLiquidity => write!(f, "insufficient liquidity"),
            OrderError::InvalidPrice => write!(f, "invalid price"),
        }
    }
}

impl std::error::Error for OrderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ETH_TOKEN;

    fn setup() -> (OrderBook, DexConfig) {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);
        let pair = Pair::new(eth, usdc);
        let book = OrderBook::new(pair);
        let config = DexConfig::default();
        (book, config)
    }

    fn test_trader(n: u8) -> Address {
        Address::repeat_byte(n)
    }

    #[test]
    fn test_place_limit_order() {
        let (mut book, config) = setup();
        let trader = test_trader(1);

        let (order_id, result) = book
            .place_limit_order(
                trader,
                OrderSide::Buy,
                Price::from_u128(100, 1),
                U256::from(1000),
                &config,
            )
            .unwrap();

        assert_eq!(result.fills.len(), 0);
        assert_eq!(result.remaining_amount, U256::from(1000));
        assert!(!result.fully_filled);

        let order = book.get_order(order_id).unwrap();
        assert_eq!(order.remaining_amount, U256::from(1000));
    }

    #[test]
    fn test_order_matching() {
        let (mut book, config) = setup();
        let buyer = test_trader(1);
        let seller = test_trader(2);

        // Place a sell order first
        book.place_limit_order(
            seller,
            OrderSide::Sell,
            Price::from_u128(100, 1),
            U256::from(1000),
            &config,
        )
        .unwrap();

        // Place a buy order that crosses the spread
        let (_, result) = book
            .place_limit_order(
                buyer,
                OrderSide::Buy,
                Price::from_u128(100, 1),
                U256::from(500),
                &config,
            )
            .unwrap();

        assert_eq!(result.fills.len(), 1);
        assert_eq!(result.fills[0].base_amount, U256::from(500));
        assert!(result.fully_filled);
    }

    #[test]
    fn test_market_order() {
        let (mut book, config) = setup();
        let maker = test_trader(1);
        let taker = test_trader(2);

        // Add liquidity
        book.place_limit_order(
            maker,
            OrderSide::Sell,
            Price::from_u128(100, 1),
            U256::from(1000),
            &config,
        )
        .unwrap();

        // Market buy
        let result = book
            .place_market_order(taker, OrderSide::Buy, U256::from(500), &config)
            .unwrap();

        assert_eq!(result.fills.len(), 1);
        assert_eq!(result.fills[0].base_amount, U256::from(500));
    }

    #[test]
    fn test_cancel_order() {
        let (mut book, config) = setup();
        let trader = test_trader(1);

        let (order_id, _) = book
            .place_limit_order(
                trader,
                OrderSide::Buy,
                Price::from_u128(100, 1),
                U256::from(1000),
                &config,
            )
            .unwrap();

        let cancelled = book.cancel_order(order_id).unwrap();
        assert_eq!(cancelled.status, crate::order::OrderStatus::Cancelled);
        assert!(book.get_order(order_id).is_none());
    }

    #[test]
    fn test_price_time_priority() {
        let (mut book, config) = setup();
        let maker1 = test_trader(1);
        let maker2 = test_trader(2);
        let taker = test_trader(3);

        // Place two sell orders at same price - first one should match first
        let (order1, _) = book
            .place_limit_order(
                maker1,
                OrderSide::Sell,
                Price::from_u128(100, 1),
                U256::from(500),
                &config,
            )
            .unwrap();

        let (order2, _) = book
            .place_limit_order(
                maker2,
                OrderSide::Sell,
                Price::from_u128(100, 1),
                U256::from(500),
                &config,
            )
            .unwrap();

        // Buy 500 - should match first order only
        let result = book
            .place_market_order(taker, OrderSide::Buy, U256::from(500), &config)
            .unwrap();

        assert_eq!(result.fills.len(), 1);
        assert_eq!(result.fills[0].maker_order_id, order1);

        // First order should be filled, second should remain
        assert!(book.get_order(order1).is_none()); // Removed when filled
        assert!(book.get_order(order2).is_some());
    }
}
