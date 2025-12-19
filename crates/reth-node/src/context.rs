//! Payload builder context with DEX transaction interception.

use crate::dex::{DexHandler, DexResult, TokenTransfer};
use crate::primitives::ExecutionInfo;
use crate::DEX_PREDEPLOY_ADDRESS;
use alloy_consensus::{transaction::Recovered, Eip658Value, Transaction, Typed2718};
use alloy_eips::Encodable2718;
use alloy_evm::{Database, EvmError};
use alloy_op_evm::block::receipt_builder::OpReceiptBuilder;
use alloy_primitives::{Bytes, U256};
use op_alloy_consensus::OpDepositReceipt;
use op_revm::transaction::abstraction::OpTransactionBuilder;
use op_revm::OpSpecId;
use reth_basic_payload_builder::PayloadConfig;
use reth_chainspec::EthChainSpec;
use reth_evm::{eth::receipt_builder::ReceiptBuilderCtx, ConfigureEvm, Evm, EvmEnv};
use reth_node_api::{PayloadBuilderAttributes, PayloadBuilderError};
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_evm::{OpEvmConfig, OpNextBlockEnvAttributes};
use reth_optimism_forks::OpHardforks;
use reth_optimism_node::OpPayloadBuilderAttributes;
use reth_optimism_payload_builder::error::OpPayloadBuilderError;
use reth_optimism_primitives::{OpReceipt, OpTransactionSigned};
use reth_primitives::SealedHeader;
use reth_primitives_traits::SignedTransaction;
use reth_revm::State;
use revm::context::result::ResultAndState;
use revm::context::tx::TxEnvBuilder;
use revm::context_interface::Block as RevmBlock;
use revm::interpreter::as_u64_saturated;
use revm::DatabaseCommit;
use std::sync::Arc;
use tracing::{debug, info, trace, warn};

/// Context for building payloads with DEX support.
#[derive(Debug)]
pub struct DexPayloadBuilderCtx {
    pub evm_config: OpEvmConfig,
    pub chain_spec: Arc<OpChainSpec>,
    pub config: PayloadConfig<OpPayloadBuilderAttributes<OpTransactionSigned>>,
    pub evm_env: EvmEnv<OpSpecId>,
    pub block_env_attributes: OpNextBlockEnvAttributes,
    pub dex_handler: Arc<DexHandler>,
}

impl DexPayloadBuilderCtx {
    pub fn parent(&self) -> &SealedHeader {
        &self.config.parent_header
    }

    pub fn attributes(&self) -> &OpPayloadBuilderAttributes<OpTransactionSigned> {
        &self.config.attributes
    }

    pub fn block_gas_limit(&self) -> u64 {
        self.attributes()
            .gas_limit
            .unwrap_or(self.evm_env.block_env.gas_limit)
    }

    pub fn block_number(&self) -> u64 {
        as_u64_saturated!(self.evm_env.block_env.number)
    }

    pub fn base_fee(&self) -> u64 {
        self.evm_env.block_env.basefee
    }

    pub fn get_blob_gasprice(&self) -> Option<u64> {
        self.evm_env
            .block_env
            .blob_gasprice()
            .map(|gasprice| gasprice as u64)
    }

    fn is_regolith_active(&self) -> bool {
        self.chain_spec
            .is_regolith_active_at_timestamp(self.attributes().timestamp())
    }

    fn is_canyon_active(&self) -> bool {
        self.chain_spec
            .is_canyon_active_at_timestamp(self.attributes().timestamp())
    }

    fn build_receipt<E: Evm>(
        &self,
        ctx: ReceiptBuilderCtx<'_, OpTransactionSigned, E>,
        deposit_nonce: Option<u64>,
    ) -> OpReceipt {
        let receipt_builder = self.evm_config.block_executor_factory().receipt_builder();
        match receipt_builder.build_receipt(ctx) {
            Ok(receipt) => receipt,
            Err(ctx) => {
                let receipt = alloy_consensus::Receipt {
                    status: Eip658Value::Eip658(ctx.result.is_success()),
                    cumulative_gas_used: ctx.cumulative_gas_used,
                    logs: ctx.result.into_logs(),
                };
                receipt_builder.build_deposit_receipt(OpDepositReceipt {
                    inner: receipt,
                    deposit_nonce,
                    deposit_receipt_version: self.is_canyon_active().then_some(1),
                })
            }
        }
    }

    fn build_dex_receipt(
        &self,
        cumulative_gas_used: u64,
        logs: Vec<alloy_primitives::Log>,
        deposit_nonce: Option<u64>,
    ) -> OpReceipt {
        let receipt = alloy_consensus::Receipt {
            status: Eip658Value::Eip658(true),
            cumulative_gas_used,
            logs,
        };

        if let Some(nonce) = deposit_nonce {
            OpReceipt::Deposit(OpDepositReceipt {
                inner: receipt,
                deposit_nonce: Some(nonce),
                deposit_receipt_version: self.is_canyon_active().then_some(1),
            })
        } else {
            OpReceipt::Eip1559(receipt)
        }
    }

    fn handle_dex_transaction<DB: Database>(
        &self,
        db: &mut State<DB>,
        tx: &Recovered<OpTransactionSigned>,
        info: &mut ExecutionInfo,
        gas_used: u64,
        deposit_nonce: Option<u64>,
    ) -> Result<(), PayloadBuilderError> {
        let sender = tx.signer();
        let calldata: Bytes = tx.input().clone();
        let value: U256 = tx.value();

        let dex_result = self
            .dex_handler
            .handle_transaction(sender, &calldata, value)
            .map_err(|e| PayloadBuilderError::Other(Box::new(DexError(e.to_string()))))?;

        // Execute token transfers via protocolTransfer calls
        let transfers = self.get_transfers(&dex_result);
        let mut all_logs = Vec::new();

        // Create EVM instance for executing token transfers
        let mut evm = self.evm_config.evm_with_env(&mut *db, self.evm_env.clone());

        for transfer in &transfers {
            if transfer.token == alloy_primitives::Address::ZERO {
                // Handle ETH transfers by executing a simple value transfer transaction
                debug!(target: "payload_builder",
                    from = ?transfer.from,
                    to = ?transfer.to,
                    amount = ?transfer.amount,
                    "Executing ETH transfer"
                );

                let deployer_account_info = evm
                    .db_mut()
                    .load_cache_account(DEX_PREDEPLOY_ADDRESS)
                    .unwrap();
                let deployer_nonce = deployer_account_info
                    .account_info()
                    .unwrap_or_default()
                    .nonce;

                // Note: ETH inbound (user -> DEX) is already handled by transaction value
                // We only need to handle outbound (DEX -> user)
                if transfer.from == DEX_PREDEPLOY_ADDRESS {
                    let tx_result = evm.transact(
                        OpTransactionBuilder::new()
                            .base(
                                TxEnvBuilder::new()
                                    .caller(transfer.from)
                                    .to(transfer.to)
                                    .nonce(deployer_nonce)
                                    .chain_id(Some(self.chain_spec.chain_id()))
                                    .value(transfer.amount),
                            )
                            .build_fill(), // .expect("Should be able to build ETH transfer tx"),
                    );

                    match tx_result {
                        Ok(result_and_state) => {
                            // Commit the ETH transfer state changes
                            evm.db_mut().commit(result_and_state.state);
                            debug!(target: "payload_builder", "ETH transfer succeeded");
                        }
                        Err(err) => {
                            warn!(target: "payload_builder",
                                ?err,
                                to = ?transfer.to,
                                amount = ?transfer.amount,
                                "ETH transfer failed"
                            );
                        }
                    }
                }
                continue;
            }

            let calldata = transfer.encode_calldata();
            debug!(target: "payload_builder",
                token = ?transfer.token,
                from = ?transfer.from,
                to = ?transfer.to,
                amount = ?transfer.amount,
                "Executing protocolTransfer"
            );

            // Execute the protocolTransfer call as a system call from the DEX address
            let result = evm.transact_system_call(DEX_PREDEPLOY_ADDRESS, transfer.token, calldata);

            match result {
                Ok(result_and_state) => {
                    // Commit the state changes
                    evm.db_mut().commit(result_and_state.state);
                    // Collect logs (Transfer events)
                    all_logs.extend(result_and_state.result.into_logs());
                    debug!(target: "payload_builder", "protocolTransfer succeeded");
                }
                Err(err) => {
                    warn!(target: "payload_builder",
                        token = ?transfer.token,
                        from = ?transfer.from,
                        to = ?transfer.to,
                        ?err,
                        "protocolTransfer failed"
                    );
                    // Continue with other transfers even if one fails
                }
            }
        }

        // Add DEX-specific logs
        let dex_logs = self.dex_handler.create_logs(&dex_result);
        all_logs.extend(dex_logs);

        info.cumulative_gas_used += gas_used;
        info.cumulative_da_bytes_used +=
            op_alloy_flz::tx_estimated_size_fjord(tx.encoded_2718().as_slice());

        let receipt = self.build_dex_receipt(info.cumulative_gas_used, all_logs, deposit_nonce);
        info.receipts.push(receipt);
        info.executed_senders.push(sender);
        info.executed_transactions.push(tx.clone().into_inner());

        Ok(())
    }

    /// Extract token transfers from a DexResult.
    fn get_transfers(&self, result: &DexResult) -> Vec<TokenTransfer> {
        match result {
            DexResult::OrderPlaced { transfers, .. } => transfers.clone(),
            DexResult::SwapExecuted { transfers, .. } => transfers.clone(),
            _ => Vec::new(),
        }
    }

    /// Execute sequencer transactions from payload attributes.
    pub fn execute_sequencer_transactions(
        &self,
        db: &mut State<impl Database>,
    ) -> Result<ExecutionInfo, PayloadBuilderError> {
        let mut info = ExecutionInfo::with_capacity(self.attributes().transactions.len());
        let mut evm = self.evm_config.evm_with_env(&mut *db, self.evm_env.clone());

        for sequencer_tx in &self.attributes().transactions {
            if sequencer_tx.value().is_eip4844() {
                return Err(PayloadBuilderError::other(
                    OpPayloadBuilderError::BlobTransactionRejected,
                ));
            }

            let sequencer_tx = sequencer_tx
                .value()
                .try_clone_into_recovered()
                .map_err(|_| {
                    PayloadBuilderError::other(OpPayloadBuilderError::TransactionEcRecoverFailed)
                })?;

            let depositor_nonce = (self.is_regolith_active() && sequencer_tx.is_deposit())
                .then(|| {
                    evm.db_mut()
                        .load_cache_account(sequencer_tx.signer())
                        .map(|acc| acc.account_info().unwrap_or_default().nonce)
                })
                .transpose()
                .map_err(|_| {
                    PayloadBuilderError::other(OpPayloadBuilderError::AccountLoadFailed(
                        sequencer_tx.signer(),
                    ))
                })?;

            // Check for DEX transaction
            if sequencer_tx.to() == Some(DEX_PREDEPLOY_ADDRESS) {
                debug!(target: "payload_builder", sender = ?sequencer_tx.signer(), "Processing DEX tx");

                // Execute EVM to get gas and state changes (nonce, balance)
                // We commit state so nonce increments - only the DEX orderbook is in-memory
                let ResultAndState { result, state } = match evm.transact(&sequencer_tx) {
                    Ok(res) => res,
                    Err(err) => {
                        warn!(target: "payload_builder", ?err, "DEX sequencer transaction EVM failed");
                        continue;
                    }
                };

                // Commit EVM state (nonce increment, gas payment)
                evm.db_mut().commit(state);

                if self
                    .handle_dex_transaction(
                        evm.db_mut(),
                        &sequencer_tx,
                        &mut info,
                        result.gas_used(),
                        depositor_nonce,
                    )
                    .is_err()
                {
                    continue;
                }
                continue;
            }

            // Regular transaction
            let ResultAndState { result, state } = match evm.transact(&sequencer_tx) {
                Ok(res) => res,
                Err(err) => {
                    if err.is_invalid_tx_err() {
                        trace!(target: "payload_builder", %err, "Skipping invalid sequencer tx");
                        continue;
                    }
                    return Err(PayloadBuilderError::EvmExecutionError(Box::new(err)));
                }
            };

            info.cumulative_gas_used += result.gas_used();

            if !sequencer_tx.is_deposit() {
                info.cumulative_da_bytes_used +=
                    op_alloy_flz::tx_estimated_size_fjord(sequencer_tx.encoded_2718().as_slice());
            }

            let ctx = ReceiptBuilderCtx {
                tx: sequencer_tx.inner(),
                evm: &evm,
                result,
                state: &state,
                cumulative_gas_used: info.cumulative_gas_used,
            };
            info.receipts.push(self.build_receipt(ctx, depositor_nonce));

            evm.db_mut().commit(state);
            info.executed_senders.push(sequencer_tx.signer());
            info.executed_transactions.push(sequencer_tx.into_inner());
        }

        Ok(info)
    }

    /// Execute best transactions from the pool.
    pub fn execute_best_transactions<I>(
        &self,
        info: &mut ExecutionInfo,
        db: &mut State<impl Database>,
        best_txs: &mut I,
    ) -> Result<(), PayloadBuilderError>
    where
        I: Iterator<Item = Recovered<OpTransactionSigned>>,
    {
        let block_gas_limit = self.block_gas_limit();
        let base_fee = self.base_fee();
        let mut evm = self.evm_config.evm_with_env(&mut *db, self.evm_env.clone());

        for tx in best_txs {
            if tx.is_eip4844() || tx.is_deposit() {
                continue;
            }

            if info.would_exceed_gas_limit(tx.gas_limit(), block_gas_limit) {
                debug!(target: "payload_builder", "Tx would exceed gas limit");
                continue;
            }

            // Check for DEX transaction
            if tx.to() == Some(DEX_PREDEPLOY_ADDRESS) {
                debug!(target: "payload_builder", sender = ?tx.signer(), "Processing DEX tx");

                // Execute EVM to get gas and state changes (nonce, balance)
                // We commit state so nonce increments - only the DEX orderbook is in-memory
                let ResultAndState { result, state } = match evm.transact(&tx) {
                    Ok(res) => res,
                    Err(err) => {
                        warn!(target: "payload_builder", ?err, "DEX pool tx EVM failed");
                        continue;
                    }
                };

                // Commit EVM state (nonce increment, gas payment)
                evm.db_mut().commit(state);

                if self
                    .handle_dex_transaction(evm.db_mut(), &tx, info, result.gas_used(), None)
                    .is_ok()
                {
                    debug!(target: "payload_builder", "DEX transaction executed");
                }
                continue;
            }

            // Regular transaction
            let ResultAndState { result, state } = match evm.transact(&tx) {
                Ok(res) => res,
                Err(err) => {
                    if err.is_invalid_tx_err() {
                        trace!(target: "payload_builder", %err, "Skipping invalid tx");
                        continue;
                    }
                    return Err(PayloadBuilderError::evm(err));
                }
            };

            let gas_used = result.gas_used();
            info.cumulative_gas_used += gas_used;
            info.cumulative_da_bytes_used +=
                op_alloy_flz::tx_estimated_size_fjord(tx.encoded_2718().as_slice());

            let ctx = ReceiptBuilderCtx {
                tx: tx.inner(),
                evm: &evm,
                result,
                state: &state,
                cumulative_gas_used: info.cumulative_gas_used,
            };
            info.receipts.push(self.build_receipt(ctx, None));

            evm.db_mut().commit(state);

            let miner_fee = tx
                .effective_tip_per_gas(base_fee)
                .expect("fee valid after execution");
            info.total_fees += U256::from(miner_fee) * U256::from(gas_used);

            info.executed_senders.push(tx.signer());
            info.executed_transactions.push(tx.into_inner());
        }

        Ok(())
    }
}

#[derive(Debug)]
struct DexError(String);

impl std::fmt::Display for DexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DexError {}
