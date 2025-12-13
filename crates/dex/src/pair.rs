//! Trading pair definitions.

use crate::types::{TokenId, U256};
use alloy::primitives::keccak256;
use std::fmt;

/// Unique identifier for a trading pair.
/// This is deterministically generated from the two token IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PairId(pub [u8; 32]);

impl PairId {
    /// Create a PairId from two tokens.
    /// The pair ID is the same regardless of token order.
    pub fn from_tokens(token_a: TokenId, token_b: TokenId) -> Self {
        // Sort tokens to ensure deterministic pair ID regardless of order
        let (first, second) = if token_a <= token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        // Hash the concatenated addresses
        let mut data = [0u8; 40];
        data[..20].copy_from_slice(first.as_slice());
        data[20..].copy_from_slice(second.as_slice());
        let hash = keccak256(&data);
        Self(hash.0)
    }
}

/// A trading pair consisting of a base token and a quote token.
///
/// Convention:
/// - Base token: The token being bought/sold (e.g., ETH in ETH/USDC)
/// - Quote token: The token used for pricing (e.g., USDC in ETH/USDC)
///
/// A buy order buys base with quote.
/// A sell order sells base for quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pair {
    /// The base token (the token being traded).
    pub base: TokenId,
    /// The quote token (the token used for pricing).
    pub quote: TokenId,
}

impl Pair {
    /// Create a new trading pair.
    pub fn new(base: TokenId, quote: TokenId) -> Self {
        Self { base, quote }
    }

    /// Get the pair ID.
    pub fn id(&self) -> PairId {
        PairId::from_tokens(self.base, self.quote)
    }

    /// Get the inverse pair (swap base and quote).
    pub fn inverse(&self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
        }
    }

    /// Check if this pair contains the given token.
    pub fn contains(&self, token: TokenId) -> bool {
        self.base == token || self.quote == token
    }

    /// Get the other token in the pair.
    pub fn other_token(&self, token: TokenId) -> Option<TokenId> {
        if self.base == token {
            Some(self.quote)
        } else if self.quote == token {
            Some(self.base)
        } else {
            None
        }
    }
}

impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// Statistics about an orderbook for a pair.
#[derive(Debug, Clone, Default)]
pub struct PairStats {
    /// Best bid price (highest buy order).
    pub best_bid: Option<U256>,
    /// Best ask price (lowest sell order).
    pub best_ask: Option<U256>,
    /// Total volume in base token.
    pub total_volume: U256,
    /// Number of active buy orders.
    pub buy_order_count: usize,
    /// Number of active sell orders.
    pub sell_order_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Address, ETH_TOKEN};

    #[test]
    fn test_pair_id_deterministic() {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);

        let pair1 = Pair::new(eth, usdc);
        let pair2 = Pair::new(usdc, eth);

        // Same underlying pair ID regardless of order
        assert_eq!(
            PairId::from_tokens(eth, usdc),
            PairId::from_tokens(usdc, eth)
        );

        // But the Pair structs are different (direction matters)
        assert_ne!(pair1, pair2);
        assert_eq!(pair1.inverse(), pair2);
    }

    #[test]
    fn test_pair_contains() {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);
        let wbtc = Address::repeat_byte(0x02);

        let pair = Pair::new(eth, usdc);

        assert!(pair.contains(eth));
        assert!(pair.contains(usdc));
        assert!(!pair.contains(wbtc));
    }

    #[test]
    fn test_other_token() {
        let eth = ETH_TOKEN;
        let usdc = Address::repeat_byte(0x01);
        let wbtc = Address::repeat_byte(0x02);

        let pair = Pair::new(eth, usdc);

        assert_eq!(pair.other_token(eth), Some(usdc));
        assert_eq!(pair.other_token(usdc), Some(eth));
        assert_eq!(pair.other_token(wbtc), None);
    }
}
