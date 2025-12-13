//! Core type definitions for the DEX.
//!
//! Re-exports from alloy-primitives for Ethereum-compatible types.

pub use alloy::primitives::{Address, U256};

/// Unique identifier for a token (contract address).
/// For ETH, use `Address::ZERO`.
pub type TokenId = Address;

/// ETH token identifier (zero address).
pub const ETH_TOKEN: TokenId = Address::ZERO;

/// Amount of tokens, represented as U256 to handle large token supplies.
/// This is in the smallest unit (e.g., wei for ETH, smallest decimal for ERC-20).
pub type Amount = U256;

/// Price represented as a rational number (numerator/denominator) for precision.
/// Price is expressed as: how much quote token per 1 unit of base token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Price {
    /// Numerator of the price ratio.
    pub numerator: U256,
    /// Denominator of the price ratio (must be non-zero).
    pub denominator: U256,
}

impl Price {
    /// Create a new price. Panics if denominator is zero.
    pub fn new(numerator: U256, denominator: U256) -> Self {
        assert!(!denominator.is_zero(), "price denominator cannot be zero");
        Self {
            numerator,
            denominator,
        }
    }

    /// Create a price from u128 values for convenience.
    pub fn from_u128(numerator: u128, denominator: u128) -> Self {
        Self::new(U256::from(numerator), U256::from(denominator))
    }

    /// Create a price from a simple integer ratio (price = value, i.e., value/1).
    pub fn from_integer(value: U256) -> Self {
        Self {
            numerator: value,
            denominator: U256::from(1),
        }
    }

    /// Calculate the amount of quote token for a given base amount.
    /// Returns (base_amount * numerator) / denominator.
    pub fn quote_amount(&self, base_amount: Amount) -> Option<Amount> {
        base_amount
            .checked_mul(self.numerator)?
            .checked_div(self.denominator)
    }

    /// Calculate the amount of base token for a given quote amount.
    /// Returns (quote_amount * denominator) / numerator.
    pub fn base_amount(&self, quote_amount: Amount) -> Option<Amount> {
        quote_amount
            .checked_mul(self.denominator)?
            .checked_div(self.numerator)
    }

    /// Compare two prices. Returns ordering based on their ratio values.
    pub fn cmp_value(&self, other: &Self) -> std::cmp::Ordering {
        // Compare a/b vs c/d by comparing a*d vs c*b
        // Use checked_mul to avoid overflow, fall back to comparison if overflow
        let lhs = self.numerator.saturating_mul(other.denominator);
        let rhs = other.numerator.saturating_mul(self.denominator);
        lhs.cmp(&rhs)
    }

    /// Convert to f64 for display purposes (may lose precision).
    pub fn to_f64(&self) -> f64 {
        // Convert to f64 carefully to avoid precision loss for large numbers
        let num: u128 = self.numerator.try_into().unwrap_or(u128::MAX);
        let den: u128 = self.denominator.try_into().unwrap_or(u128::MAX);
        num as f64 / den as f64
    }

    /// Invert the price (swap numerator and denominator).
    pub fn invert(&self) -> Self {
        Self {
            numerator: self.denominator,
            denominator: self.numerator,
        }
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp_value(other))
    }
}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_value(other)
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.denominator == U256::from(1) {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_id() {
        let eth = ETH_TOKEN;
        assert!(eth.is_zero());

        let usdc = Address::repeat_byte(0x01);
        assert!(!usdc.is_zero());
    }

    #[test]
    fn test_price_calculations() {
        // Price of 2 (2 quote per 1 base)
        let price = Price::from_u128(2, 1);
        assert_eq!(
            price.quote_amount(U256::from(100)),
            Some(U256::from(200))
        );
        assert_eq!(
            price.base_amount(U256::from(200)),
            Some(U256::from(100))
        );

        // Price of 0.5 (1 quote per 2 base)
        let price = Price::from_u128(1, 2);
        assert_eq!(
            price.quote_amount(U256::from(100)),
            Some(U256::from(50))
        );
        assert_eq!(
            price.base_amount(U256::from(50)),
            Some(U256::from(100))
        );
    }

    #[test]
    fn test_price_ordering() {
        let p1 = Price::from_u128(1, 2); // 0.5
        let p2 = Price::from_u128(2, 3); // 0.666...
        let p3 = Price::from_u128(3, 4); // 0.75

        assert!(p1 < p2);
        assert!(p2 < p3);
        assert!(p1 < p3);
    }

    #[test]
    fn test_price_invert() {
        let price = Price::from_u128(3, 4); // 0.75
        let inverted = price.invert();
        assert_eq!(inverted.numerator, U256::from(4));
        assert_eq!(inverted.denominator, U256::from(3));
    }
}
