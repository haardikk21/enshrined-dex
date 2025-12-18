//! Primitive types for payload building.

use alloy_primitives::{Address, U256};
use reth_optimism_primitives::{OpReceipt, OpTransactionSigned};

/// Execution information collected during payload building.
#[derive(Default, Debug)]
pub struct ExecutionInfo {
    pub executed_transactions: Vec<OpTransactionSigned>,
    pub executed_senders: Vec<Address>,
    pub receipts: Vec<OpReceipt>,
    pub cumulative_gas_used: u64,
    pub cumulative_da_bytes_used: u64,
    pub total_fees: U256,
}

impl ExecutionInfo {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            executed_transactions: Vec::with_capacity(capacity),
            executed_senders: Vec::with_capacity(capacity),
            receipts: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn would_exceed_gas_limit(&self, tx_gas_limit: u64, block_gas_limit: u64) -> bool {
        self.cumulative_gas_used + tx_gas_limit > block_gas_limit
    }
}
