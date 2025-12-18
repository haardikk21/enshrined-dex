//! DEX-aware payload job generator.

use crate::dex::DexHandler;
use crate::job::DexPayloadJob;
use crate::payload::DexOpPayloadBuilder;
use alloy_eips::BlockNumberOrTag;
use reth_basic_payload_builder::PayloadConfig;
use reth_node_api::PayloadBuilderAttributes;
use reth_optimism_evm::OpEvmConfig;
use reth_optimism_node::OpPayloadBuilderAttributes;
use reth_optimism_primitives::OpTransactionSigned;
use reth_payload_builder::{PayloadBuilderError, PayloadJobGenerator};
use reth_primitives_traits::Block;
use reth_provider::{BlockReaderIdExt, BlockSource, ChainSpecProvider, StateProviderFactory};
use reth_transaction_pool::TransactionPool;
use std::sync::Arc;
use tracing::info;

/// Generator that creates DEX-aware payload jobs.
#[derive(Debug)]
pub struct DexPayloadJobGenerator<Pool, Client> {
    client: Client,
    pool: Pool,
    evm_config: OpEvmConfig,
    dex_handler: Arc<DexHandler>,
}

impl<Pool, Client> DexPayloadJobGenerator<Pool, Client> {
    pub fn new(client: Client, pool: Pool, evm_config: OpEvmConfig) -> Self {
        info!(target: "payload_builder", "Creating DEX payload job generator");
        Self {
            client,
            pool,
            evm_config,
            dex_handler: Arc::new(DexHandler::new()),
        }
    }
}

impl<Pool, Client> PayloadJobGenerator for DexPayloadJobGenerator<Pool, Client>
where
    Pool: TransactionPool<
            Transaction: reth_optimism_txpool::OpPooledTx<
                Consensus = op_alloy_consensus::OpTxEnvelope,
            >,
        > + Clone
        + Unpin
        + 'static,
    Client: StateProviderFactory
        + ChainSpecProvider<ChainSpec = reth_optimism_chainspec::OpChainSpec>
        + BlockReaderIdExt<Header = alloy_consensus::Header>
        + Clone
        + Unpin
        + 'static,
{
    type Job = DexPayloadJob<Pool, Client>;

    fn new_payload_job(
        &self,
        attributes: OpPayloadBuilderAttributes<OpTransactionSigned>,
    ) -> Result<Self::Job, PayloadBuilderError> {
        let parent_hash = attributes.parent();

        info!(
            target: "payload_builder",
            parent = ?parent_hash,
            timestamp = attributes.timestamp(),
            "Creating new DEX payload job"
        );

        let parent_block = if parent_hash.is_zero() {
            self.client
                .block_by_number_or_tag(BlockNumberOrTag::Latest)?
                .ok_or_else(|| PayloadBuilderError::MissingParentBlock(parent_hash))?
                .seal_slow()
        } else {
            self.client
                .find_block_by_hash(parent_hash, BlockSource::Any)?
                .ok_or_else(|| PayloadBuilderError::MissingParentBlock(parent_hash))?
                .seal_unchecked(parent_hash)
        };

        let config = PayloadConfig::new(Arc::new(parent_block.clone_sealed_header()), attributes);

        let builder = DexOpPayloadBuilder::new(
            self.pool.clone(),
            self.client.clone(),
            self.evm_config.clone(),
            Arc::clone(&self.dex_handler),
        );

        Ok(DexPayloadJob::new(config, builder))
    }
}
