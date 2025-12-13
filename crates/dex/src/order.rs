//! Order types and management.

use crate::types::{Address, Amount, Price, U256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

impl OrderId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Side of the order (buy or sell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderSide {
    /// Buy order: wants to buy base token with quote token.
    Buy,
    /// Sell order: wants to sell base token for quote token.
    Sell,
}

/// Type of order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Limit order: execute at specified price or better.
    Limit,
    /// Market order: execute immediately at best available price.
    Market,
}

/// Status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// Order is open and can be matched.
    Open,
    /// Order has been partially filled.
    PartiallyFilled,
    /// Order has been completely filled.
    Filled,
    /// Order has been cancelled.
    Cancelled,
}

/// An order in the orderbook.
#[derive(Debug, Clone)]
pub struct Order {
    /// Unique order identifier.
    pub id: OrderId,
    /// Address of the trader who placed the order.
    pub trader: Address,
    /// Buy or sell.
    pub side: OrderSide,
    /// Limit or market order.
    pub order_type: OrderType,
    /// Price per unit (for limit orders).
    pub price: Price,
    /// Original amount of base token.
    pub original_amount: Amount,
    /// Remaining amount of base token to be filled.
    pub remaining_amount: Amount,
    /// Current status of the order.
    pub status: OrderStatus,
    /// Timestamp when the order was created (unix millis).
    pub timestamp: u64,
}

impl Order {
    /// Create a new limit order.
    pub fn new_limit(
        id: OrderId,
        trader: Address,
        side: OrderSide,
        price: Price,
        amount: Amount,
    ) -> Self {
        Self {
            id,
            trader,
            side,
            order_type: OrderType::Limit,
            price,
            original_amount: amount,
            remaining_amount: amount,
            status: OrderStatus::Open,
            timestamp: current_timestamp(),
        }
    }

    /// Create a new market order.
    pub fn new_market(id: OrderId, trader: Address, side: OrderSide, amount: Amount) -> Self {
        // Market orders use a placeholder price; they match at the best available price.
        let price = match side {
            OrderSide::Buy => Price::new(U256::MAX, U256::from(1)),  // Willing to pay any price
            OrderSide::Sell => Price::new(U256::from(1), U256::MAX), // Willing to accept any price
        };
        Self {
            id,
            trader,
            side,
            order_type: OrderType::Market,
            price,
            original_amount: amount,
            remaining_amount: amount,
            status: OrderStatus::Open,
            timestamp: current_timestamp(),
        }
    }

    /// Check if the order is still active (can be matched).
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Open | OrderStatus::PartiallyFilled)
    }

    /// Fill some amount of the order.
    pub fn fill(&mut self, amount: Amount) {
        self.remaining_amount = self.remaining_amount.saturating_sub(amount);
        if self.remaining_amount.is_zero() {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }
    }

    /// Cancel the order.
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
    }

    /// Get the filled amount.
    pub fn filled_amount(&self) -> Amount {
        self.original_amount.saturating_sub(self.remaining_amount)
    }

    /// Check if this order can match with another order.
    /// For a buy order to match a sell order: buy_price >= sell_price
    /// For a sell order to match a buy order: sell_price <= buy_price
    pub fn can_match(&self, other: &Order) -> bool {
        if self.side == other.side {
            return false;
        }

        match self.side {
            OrderSide::Buy => self.price >= other.price,
            OrderSide::Sell => self.price <= other.price,
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address() -> Address {
        Address::repeat_byte(0x01)
    }

    #[test]
    fn test_order_creation() {
        let order = Order::new_limit(
            OrderId(1),
            test_address(),
            OrderSide::Buy,
            Price::from_u128(100, 1),
            U256::from(1000),
        );

        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.remaining_amount, U256::from(1000));
        assert!(order.is_active());
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new_limit(
            OrderId(1),
            test_address(),
            OrderSide::Buy,
            Price::from_u128(100, 1),
            U256::from(1000),
        );

        order.fill(U256::from(400));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining_amount, U256::from(600));
        assert_eq!(order.filled_amount(), U256::from(400));

        order.fill(U256::from(600));
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.remaining_amount, U256::ZERO);
        assert!(!order.is_active());
    }

    #[test]
    fn test_order_matching() {
        let buy_order = Order::new_limit(
            OrderId(1),
            test_address(),
            OrderSide::Buy,
            Price::from_u128(100, 1),
            U256::from(1000),
        );

        let sell_order_good = Order::new_limit(
            OrderId(2),
            test_address(),
            OrderSide::Sell,
            Price::from_u128(95, 1),
            U256::from(1000),
        );

        let sell_order_bad = Order::new_limit(
            OrderId(3),
            test_address(),
            OrderSide::Sell,
            Price::from_u128(105, 1),
            U256::from(1000),
        );

        assert!(buy_order.can_match(&sell_order_good)); // 100 >= 95
        assert!(!buy_order.can_match(&sell_order_bad)); // 100 < 105
    }
}
