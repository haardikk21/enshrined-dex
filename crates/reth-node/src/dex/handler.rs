//! DEX transaction handler.

use super::types::{DexError, DexResult};
use crate::selectors::{selectors, EnshrinedDEX};
use crate::DEX_PREDEPLOY_ADDRESS;
use alloy_primitives::{Address, Bytes, Log, B256, U256};
use alloy_sol_types::{SolEvent, SolValue};
use dex::{OrderSide, PoolManager, Price};
use parking_lot::RwLock;
use tracing::{debug, info};

/// Handler for enshrined DEX operations.
#[derive(Debug)]
pub struct DexHandler {
    pool_manager: RwLock<PoolManager>,
}

impl DexHandler {
    /// Create a new DexHandler.
    pub fn new() -> Self {
        Self {
            pool_manager: RwLock::new(PoolManager::new()),
        }
    }

    /// Handle a transaction to the DEX predeploy.
    ///
    /// # Arguments
    /// * `caller` - The address calling the DEX
    /// * `calldata` - The transaction calldata
    /// * `value` - ETH value sent with the transaction
    ///
    /// # Returns
    /// * `Ok(DexResult)` - The result of the operation
    /// * `Err(DexError)` - If the operation failed
    pub fn handle_transaction(
        &self,
        caller: Address,
        calldata: &Bytes,
        value: U256,
    ) -> Result<DexResult, DexError> {
        if calldata.len() < 4 {
            return Err(DexError::InvalidCalldata(
                "calldata too short for function selector".to_string(),
            ));
        }

        let selector = &calldata[0..4];

        match selector {
            s if s == selectors::CREATE_PAIR.as_slice() => {
                self.handle_create_pair(caller, &calldata[4..])
            }
            s if s == selectors::PLACE_LIMIT_ORDER.as_slice() => {
                self.handle_place_limit_order(caller, &calldata[4..], value)
            }
            s if s == selectors::CANCEL_ORDER.as_slice() => {
                self.handle_cancel_order(caller, &calldata[4..])
            }
            s if s == selectors::SWAP.as_slice() => self.handle_swap(caller, &calldata[4..], value),
            s if s == selectors::GET_QUOTE.as_slice() => self.handle_get_quote(&calldata[4..]),
            _ => Err(DexError::InvalidCalldata(format!(
                "unknown function selector: 0x{}",
                hex::encode(selector)
            ))),
        }
    }

    /// Handle createPair(address,address)
    fn handle_create_pair(&self, _caller: Address, data: &[u8]) -> Result<DexResult, DexError> {
        info!("DEX Handler: createPair called");

        let (token0, token1): (Address, Address) =
            <(Address, Address)>::abi_decode(data).map_err(|e| {
                DexError::InvalidCalldata(format!("failed to decode createPair: {}", e))
            })?;

        debug!("createPair: token0={:?}, token1={:?}", token0, token1);

        let mut pm = self.pool_manager.write();
        let pair = pm.create_pair(token0, token1).map_err(DexError::from)?;

        let pair_id = pair.id();
        let pair_id_bytes = B256::from_slice(&pair_id.0);

        info!(
            "Pair created: token0={:?}, token1={:?}, pair_id={:?}",
            token0, token1, pair_id_bytes
        );

        Ok(DexResult::PairCreated {
            token0,
            token1,
            pair_id: pair_id_bytes,
        })
    }

    /// Handle placeLimitOrder(address,address,bool,uint256,uint256,uint256)
    fn handle_place_limit_order(
        &self,
        caller: Address,
        data: &[u8],
        _value: U256,
    ) -> Result<DexResult, DexError> {
        let (token_in, token_out, is_buy, amount, price_num, price_denom): (
            Address,
            Address,
            bool,
            U256,
            U256,
            U256,
        ) = <(Address, Address, bool, U256, U256, U256)>::abi_decode(data).map_err(|e| {
            DexError::InvalidCalldata(format!("failed to decode placeLimitOrder: {}", e))
        })?;

        if amount == U256::ZERO {
            return Err(DexError::InvalidAmount(amount));
        }

        if price_num == U256::ZERO || price_denom == U256::ZERO {
            return Err(DexError::InvalidPrice {
                num: price_num,
                denom: price_denom,
            });
        }

        let price_num_u128: u128 = price_num.try_into().map_err(|_| DexError::InvalidPrice {
            num: price_num,
            denom: price_denom,
        })?;
        let price_denom_u128: u128 =
            price_denom.try_into().map_err(|_| DexError::InvalidPrice {
                num: price_num,
                denom: price_denom,
            })?;

        let price = Price::from_u128(price_num_u128, price_denom_u128);
        let side = if is_buy {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        let mut pm = self.pool_manager.write();
        let (order_id, _trade_result) = pm
            .place_limit_order(token_in, token_out, caller, side, price, amount)
            .map_err(DexError::from)?;

        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&order_id.0.to_be_bytes());
        let order_id_bytes = B256::from(bytes);

        Ok(DexResult::OrderPlaced {
            order_id: order_id_bytes,
            trader: caller,
            token_in,
            token_out,
            is_buy,
            amount,
            price_num,
            price_denom,
        })
    }

    /// Handle cancelOrder(bytes32)
    fn handle_cancel_order(&self, caller: Address, data: &[u8]) -> Result<DexResult, DexError> {
        let order_id: B256 = <B256>::abi_decode(data).map_err(|e| {
            DexError::InvalidCalldata(format!("failed to decode cancelOrder: {}", e))
        })?;

        // TODO: Implement proper order cancellation with order tracking
        Ok(DexResult::OrderCancelled {
            order_id,
            trader: caller,
        })
    }

    /// Handle swap(address,address,uint256,uint256)
    fn handle_swap(
        &self,
        caller: Address,
        data: &[u8],
        _value: U256,
    ) -> Result<DexResult, DexError> {
        let (token_in, token_out, amount_in, min_amount_out): (Address, Address, U256, U256) =
            <(Address, Address, U256, U256)>::abi_decode(data)
                .map_err(|e| DexError::InvalidCalldata(format!("failed to decode swap: {}", e)))?;

        if amount_in == U256::ZERO {
            return Err(DexError::InvalidAmount(amount_in));
        }

        let mut pm = self.pool_manager.write();
        let result = pm
            .execute_swap(caller, token_in, token_out, amount_in, min_amount_out)
            .map_err(DexError::from)?;

        // Convert route to Vec<B256>
        let route: Vec<B256> = result
            .route
            .hops
            .iter()
            .map(|hop| {
                let pair_id = hop.pair.id();
                B256::from_slice(&pair_id.0)
            })
            .collect();

        Ok(DexResult::SwapExecuted {
            trader: caller,
            token_in,
            token_out,
            amount_in,
            amount_out: result.amount_out,
            route,
        })
    }

    /// Handle getQuote(address,address,uint256)
    fn handle_get_quote(&self, data: &[u8]) -> Result<DexResult, DexError> {
        let (token_in, token_out, amount_in): (Address, Address, U256) =
            <(Address, Address, U256)>::abi_decode(data).map_err(|e| {
                DexError::InvalidCalldata(format!("failed to decode getQuote: {}", e))
            })?;

        let pm = self.pool_manager.read();
        let result = pm
            .get_quote(token_in, token_out, amount_in)
            .map_err(DexError::from)?;

        let route: Vec<B256> = result
            .route
            .hops
            .iter()
            .map(|hop| {
                let pair_id = hop.pair.id();
                B256::from_slice(&pair_id.0)
            })
            .collect();

        Ok(DexResult::Quote {
            amount_out: result.amount_out,
            route,
        })
    }

    /// Create logs for a DEX operation result.
    pub fn create_logs(&self, result: &DexResult) -> Vec<Log> {
        let mut logs = Vec::new();

        match result {
            DexResult::PairCreated {
                token0,
                token1,
                pair_id,
            } => {
                logs.push(Log {
                    address: DEX_PREDEPLOY_ADDRESS,
                    data: alloy_primitives::LogData::new_unchecked(
                        vec![
                            EnshrinedDEX::PairCreated::SIGNATURE_HASH.into(),
                            B256::left_padding_from(token0.as_slice()),
                            B256::left_padding_from(token1.as_slice()),
                            *pair_id,
                        ],
                        Bytes::new(),
                    ),
                });
            }
            DexResult::OrderPlaced {
                order_id,
                trader,
                token_in,
                token_out,
                is_buy,
                amount,
                price_num,
                price_denom,
            } => {
                // Non-indexed params: (address tokenOut, bool isBuy, uint256 amount, uint256 priceNum, uint256 priceDenom)
                let data = (*token_out, *is_buy, *amount, *price_num, *price_denom).abi_encode();
                logs.push(Log {
                    address: DEX_PREDEPLOY_ADDRESS,
                    data: alloy_primitives::LogData::new_unchecked(
                        vec![
                            EnshrinedDEX::LimitOrderPlaced::SIGNATURE_HASH.into(),
                            *order_id,
                            B256::left_padding_from(trader.as_slice()),
                            B256::left_padding_from(token_in.as_slice()),
                        ],
                        data.into(),
                    ),
                });
            }
            DexResult::OrderCancelled { order_id, trader } => {
                logs.push(Log {
                    address: DEX_PREDEPLOY_ADDRESS,
                    data: alloy_primitives::LogData::new_unchecked(
                        vec![
                            EnshrinedDEX::OrderCancelled::SIGNATURE_HASH.into(),
                            *order_id,
                            B256::left_padding_from(trader.as_slice()),
                        ],
                        Bytes::new(),
                    ),
                });
            }
            DexResult::SwapExecuted {
                trader,
                token_in,
                token_out,
                amount_in,
                amount_out,
                route,
            } => {
                // Non-indexed params: (uint256 amountIn, uint256 amountOut, bytes32[] route)
                let data = (*amount_in, *amount_out, route.as_slice()).abi_encode();
                logs.push(Log {
                    address: DEX_PREDEPLOY_ADDRESS,
                    data: alloy_primitives::LogData::new_unchecked(
                        vec![
                            EnshrinedDEX::Swap::SIGNATURE_HASH.into(),
                            B256::left_padding_from(trader.as_slice()),
                            B256::left_padding_from(token_in.as_slice()),
                            B256::left_padding_from(token_out.as_slice()),
                        ],
                        data.into(),
                    ),
                });
            }
            DexResult::Quote { .. } => {}
        }

        logs
    }
}

impl Default for DexHandler {
    fn default() -> Self {
        Self::new()
    }
}
