//! Type definitions for DEX operations.

use alloy_primitives::{Address, B256, U256};

/// Result of a DEX operation.
#[derive(Debug, Clone)]
pub enum DexResult {
    PairCreated {
        token0: Address,
        token1: Address,
        pair_id: B256,
    },
    OrderPlaced {
        order_id: B256,
        trader: Address,
        token_in: Address,
        token_out: Address,
        is_buy: bool,
        amount: U256,
        price_num: U256,
        price_denom: U256,
    },
    OrderCancelled {
        order_id: B256,
        trader: Address,
    },
    SwapExecuted {
        trader: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out: U256,
        route: Vec<B256>,
    },
    #[allow(dead_code)]
    Quote {
        amount_out: U256,
        route: Vec<B256>,
    },
}

/// Errors that can occur during DEX operations.
#[derive(Debug, thiserror::Error)]
pub enum DexError {
    #[error("Invalid calldata: {0}")]
    InvalidCalldata(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(U256),

    #[error("Invalid price: num={num}, denom={denom}")]
    InvalidPrice { num: U256, denom: U256 },

    #[error("DEX error: {0}")]
    DexLibrary(String),
}

impl From<dex::PoolError> for DexError {
    fn from(err: dex::PoolError) -> Self {
        DexError::DexLibrary(err.to_string())
    }
}
