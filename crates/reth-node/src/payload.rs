//! DEX-aware payload builder.

use crate::context::DexPayloadBuilderCtx;
use crate::dex::DexHandler;
use alloy_consensus::transaction::Recovered;
use alloy_consensus::{
    constants::EMPTY_WITHDRAWALS, proofs, BlockBody, Header, Transaction, Typed2718,
    EMPTY_OMMER_ROOT_HASH,
};
use alloy_eips::{eip7685::EMPTY_REQUESTS_HASH, merge::BEACON_NONCE};
use alloy_primitives::U256;
use reth_basic_payload_builder::{
    BuildArguments, BuildOutcome, MissingPayloadBehaviour, PayloadBuilder, PayloadConfig,
};
use reth_chain_state::ExecutedBlock;
use reth_chainspec::EthereumHardforks;
use reth_evm::execute::BlockBuilder;
use reth_evm::ConfigureEvm;
use reth_node_api::{PayloadBuilderAttributes, PayloadBuilderError};
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_consensus::{calculate_receipt_root_no_memo_optimism, isthmus};
use reth_optimism_evm::{OpEvmConfig, OpNextBlockEnvAttributes};
use reth_optimism_forks::OpHardforks;
use reth_optimism_node::{OpBuiltPayload, OpPayloadBuilderAttributes};
use reth_optimism_primitives::OpTransactionSigned;
use reth_primitives::RecoveredBlock;
use reth_primitives_traits::Block as _;
use reth_provider::{
    ChainSpecProvider, ExecutionOutcome, HashedPostStateProvider, StateProviderFactory,
    StateRootProvider,
};
use reth_revm::{
    database::StateProviderDatabase, db::states::bundle_state::BundleRetention, State,
};
use reth_transaction_pool::{BestTransactionsAttributes, PoolTransaction, TransactionPool};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// DEX-aware Optimism payload builder.
#[derive(Debug, Clone)]
pub struct DexOpPayloadBuilder<Pool, Client> {
    pub evm_config: OpEvmConfig,
    pub pool: Pool,
    pub client: Client,
    pub dex_handler: Arc<DexHandler>,
}

impl<Pool, Client> DexOpPayloadBuilder<Pool, Client> {
    pub fn new(
        pool: Pool,
        client: Client,
        evm_config: OpEvmConfig,
        dex_handler: Arc<DexHandler>,
    ) -> Self {
        Self {
            evm_config,
            pool,
            client,
            dex_handler,
        }
    }
}

impl<Pool, Client> PayloadBuilder for DexOpPayloadBuilder<Pool, Client>
where
    Pool: TransactionPool<
            Transaction: reth_optimism_txpool::OpPooledTx<
                Consensus = op_alloy_consensus::OpTxEnvelope,
            >,
        > + Unpin
        + 'static,
    Client:
        StateProviderFactory + ChainSpecProvider<ChainSpec = OpChainSpec> + Clone + Unpin + 'static,
{
    type Attributes = OpPayloadBuilderAttributes<OpTransactionSigned>;
    type BuiltPayload = OpBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Self::Attributes, Self::BuiltPayload>,
    ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
        let BuildArguments {
            mut cached_reads,
            config,
            ..
        } = args;

        let chain_spec = self.client.chain_spec();
        let timestamp = config.attributes.timestamp();

        let extra_data = if chain_spec.is_holocene_active_at_timestamp(timestamp) {
            config
                .attributes
                .get_holocene_extra_data(chain_spec.base_fee_params_at_timestamp(timestamp))
                .map_err(PayloadBuilderError::other)?
        } else {
            Default::default()
        };

        let block_env_attributes = OpNextBlockEnvAttributes {
            timestamp,
            suggested_fee_recipient: config.attributes.suggested_fee_recipient(),
            prev_randao: config.attributes.prev_randao(),
            gas_limit: config
                .attributes
                .gas_limit
                .unwrap_or(config.parent_header.gas_limit),
            parent_beacon_block_root: config
                .attributes
                .payload_attributes
                .parent_beacon_block_root,
            extra_data: extra_data.clone(),
        };

        let evm_env = self
            .evm_config
            .next_evm_env(&config.parent_header, &block_env_attributes)
            .map_err(PayloadBuilderError::other)?;

        let ctx = DexPayloadBuilderCtx {
            evm_config: self.evm_config.clone(),
            chain_spec: chain_spec.clone(),
            config: config.clone(),
            evm_env,
            block_env_attributes,
            dex_handler: Arc::clone(&self.dex_handler),
        };

        let state_provider = self.client.state_by_block_hash(ctx.parent().hash())?;
        let db = StateProviderDatabase::new(&state_provider);

        let mut state = State::builder()
            .with_database(cached_reads.as_db_mut(db))
            .with_bundle_update()
            .build();

        debug!(
            target: "payload_builder",
            parent = ?ctx.parent().hash(),
            parent_number = ctx.parent().number,
            "Building payload"
        );

        // Apply pre-execution changes
        self.evm_config
            .builder_for_next_block(&mut state, ctx.parent(), ctx.block_env_attributes.clone())
            .map_err(PayloadBuilderError::other)?
            .apply_pre_execution_changes()
            .map_err(PayloadBuilderError::other)?;

        // Execute sequencer transactions
        let mut info = ctx.execute_sequencer_transactions(&mut state)?;

        // Execute pool transactions if allowed
        if !ctx.attributes().no_tx_pool {
            let best_txs_attrs =
                BestTransactionsAttributes::new(ctx.base_fee(), ctx.get_blob_gasprice());
            let best_txs = self.pool.best_transactions_with_attributes(best_txs_attrs);

            let block_gas_limit = ctx.block_gas_limit();
            let mut recovered_txs: Vec<Recovered<OpTransactionSigned>> = Vec::new();

            for pool_tx in best_txs {
                let tx = pool_tx.transaction.clone().into_consensus();
                if tx.is_eip4844() || tx.is_deposit() {
                    continue;
                }
                if info.would_exceed_gas_limit(tx.gas_limit(), block_gas_limit) {
                    continue;
                }
                recovered_txs.push(tx);
            }

            ctx.execute_best_transactions(&mut info, &mut state, &mut recovered_txs.drain(..))?;
        }

        state.merge_transitions(BundleRetention::Reverts);

        let block_number = ctx.block_number();

        let execution_outcome = ExecutionOutcome::new(
            state.take_bundle(),
            vec![info.receipts.clone()],
            block_number,
            Vec::new(),
        );

        let receipts_root = execution_outcome
            .generic_receipts_root_slow(block_number, |receipts| {
                calculate_receipt_root_no_memo_optimism(
                    receipts,
                    &ctx.chain_spec,
                    ctx.attributes().timestamp(),
                )
            })
            .expect("Number is in range");

        let logs_bloom = execution_outcome
            .block_logs_bloom(block_number)
            .expect("Number is in range");

        let hashed_state = state_provider.hashed_post_state(execution_outcome.state());
        let (state_root, trie_output) = state_provider
            .state_root_with_updates(hashed_state.clone())
            .inspect_err(|err| {
                warn!(target: "payload_builder", %err, "Failed to calculate state root");
            })?;

        let (withdrawals_root, requests_hash) = if ctx
            .chain_spec
            .is_isthmus_active_at_timestamp(ctx.attributes().timestamp())
        {
            (
                Some(
                    isthmus::withdrawals_root(execution_outcome.state(), &state_provider)
                        .map_err(PayloadBuilderError::other)?,
                ),
                Some(EMPTY_REQUESTS_HASH),
            )
        } else if ctx
            .chain_spec
            .is_canyon_active_at_timestamp(ctx.attributes().timestamp())
        {
            (Some(EMPTY_WITHDRAWALS), None)
        } else {
            (None, None)
        };

        let transactions_root = proofs::calculate_transaction_root(&info.executed_transactions);

        let header = Header {
            parent_hash: ctx.parent().hash(),
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: ctx.evm_env.block_env.beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            withdrawals_root,
            logs_bloom,
            timestamp: ctx.attributes().payload_attributes.timestamp,
            mix_hash: ctx.attributes().payload_attributes.prev_randao,
            nonce: BEACON_NONCE.into(),
            base_fee_per_gas: Some(ctx.base_fee()),
            number: ctx.parent().number + 1,
            gas_limit: ctx.block_gas_limit(),
            difficulty: U256::ZERO,
            gas_used: info.cumulative_gas_used,
            extra_data,
            parent_beacon_block_root: ctx.attributes().payload_attributes.parent_beacon_block_root,
            blob_gas_used: Some(0),
            excess_blob_gas: Some(0),
            requests_hash,
        };

        let withdrawals = ctx
            .chain_spec
            .is_shanghai_active_at_timestamp(ctx.attributes().timestamp())
            .then(|| ctx.attributes().payload_attributes.withdrawals.clone());

        let block = alloy_consensus::Block::<OpTransactionSigned>::new(
            header,
            BlockBody {
                transactions: info.executed_transactions.clone(),
                ommers: vec![],
                withdrawals,
            },
        );

        let sealed_block = Arc::new(block.seal_slow());

        debug!(
            target: "payload_builder",
            block_hash = ?sealed_block.hash(),
            tx_count = info.executed_transactions.len(),
            gas_used = info.cumulative_gas_used,
            "Built payload"
        );

        let executed = ExecutedBlock {
            recovered_block: Arc::new(RecoveredBlock::new_sealed(
                sealed_block.as_ref().clone(),
                info.executed_senders,
            )),
            execution_output: Arc::new(execution_outcome),
            hashed_state: Arc::new(hashed_state),
            trie_updates: Arc::new(trie_output),
        };

        let payload = OpBuiltPayload::new(
            config.attributes.payload_id(),
            sealed_block,
            info.total_fees,
            Some(executed),
        );

        Ok(BuildOutcome::Better {
            payload,
            cached_reads,
        })
    }

    fn on_missing_payload(
        &self,
        _args: BuildArguments<Self::Attributes, Self::BuiltPayload>,
    ) -> MissingPayloadBehaviour<Self::BuiltPayload> {
        MissingPayloadBehaviour::AwaitInProgress
    }

    fn build_empty_payload(
        &self,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<Self::BuiltPayload, PayloadBuilderError> {
        self.try_build(BuildArguments {
            config,
            cached_reads: Default::default(),
            cancel: Default::default(),
            best_payload: None,
        })?
        .into_payload()
        .ok_or(PayloadBuilderError::MissingPayload)
    }
}
