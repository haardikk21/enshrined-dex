//! Type definitions for DEX operations.

use crate::selectors::DexToken;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_sol_types::SolCall;

/// A token transfer to be executed via protocolTransfer.
#[derive(Debug, Clone)]
pub struct TokenTransfer {
    /// The token contract address (address(0) for ETH).
    pub token: Address,
    /// The sender address.
    pub from: Address,
    /// The recipient address.
    pub to: Address,
    /// The amount to transfer.
    pub amount: U256,
}

impl TokenTransfer {
    /// Encode this transfer as protocolTransfer calldata.
    /// protocolTransfer(address from, address to, uint256 amount)
    pub fn encode_calldata(&self) -> Bytes {
        let call = DexToken::protocolTransferCall {
            from: self.from,
            to: self.to,
            amount: self.amount,
        };
        call.abi_encode().into()
    }
}

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
        /// Token transfers for escrowing collateral.
        transfers: Vec<TokenTransfer>,
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
        /// Token transfers for the swap.
        transfers: Vec<TokenTransfer>,
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
