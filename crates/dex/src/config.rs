//! DEX configuration parameters.

/// Configuration for the DEX.
#[derive(Debug, Clone)]
pub struct DexConfig {
    /// Fee charged per trade in basis points (1 bp = 0.01%).
    /// For example, 30 = 0.30% fee.
    pub fee_bps: u32,

    /// Maximum number of hops allowed when routing through multiple pairs.
    pub max_routing_hops: usize,

    /// Minimum order size in the smallest token unit.
    /// Orders below this size will be rejected.
    pub min_order_size: u128,

    /// Whether to allow self-trading (same address on both sides).
    pub allow_self_trade: bool,
}

impl Default for DexConfig {
    fn default() -> Self {
        Self {
            fee_bps: 30,           // 0.30% default fee
            max_routing_hops: 3,   // Max 3 hops (4 tokens in path)
            min_order_size: 1,     // Minimum 1 unit
            allow_self_trade: false,
        }
    }
}

impl DexConfig {
    /// Create a new configuration with custom fee.
    pub fn with_fee_bps(mut self, fee_bps: u32) -> Self {
        self.fee_bps = fee_bps;
        self
    }

    /// Create a new configuration with custom max routing hops.
    pub fn with_max_routing_hops(mut self, max_hops: usize) -> Self {
        self.max_routing_hops = max_hops;
        self
    }

    /// Create a new configuration with custom minimum order size.
    pub fn with_min_order_size(mut self, min_size: u128) -> Self {
        self.min_order_size = min_size;
        self
    }

    /// Create a new configuration allowing self-trading.
    pub fn with_self_trade(mut self, allow: bool) -> Self {
        self.allow_self_trade = allow;
        self
    }

    /// Calculate the fee amount for a given trade amount.
    /// Returns the fee amount (to be subtracted from the output).
    pub fn calculate_fee(&self, amount: u128) -> u128 {
        // fee = amount * fee_bps / 10000
        amount.saturating_mul(self.fee_bps as u128) / 10000
    }

    /// Calculate the amount after fee deduction.
    pub fn amount_after_fee(&self, amount: u128) -> u128 {
        amount.saturating_sub(self.calculate_fee(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        let config = DexConfig::default(); // 30 bps = 0.30%

        // 10000 * 30 / 10000 = 30
        assert_eq!(config.calculate_fee(10000), 30);

        // Amount after fee: 10000 - 30 = 9970
        assert_eq!(config.amount_after_fee(10000), 9970);
    }

    #[test]
    fn test_custom_fee() {
        let config = DexConfig::default().with_fee_bps(100); // 1%

        assert_eq!(config.calculate_fee(10000), 100);
        assert_eq!(config.amount_after_fee(10000), 9900);
    }
}
