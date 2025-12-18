//! Custom Optimism Reth node with enshrined DEX support.
//!
//! This node intercepts transactions to the DEX predeploy address and executes
//! them through the in-memory DEX library instead of EVM.
//!
//! Run with: `cargo run -p reth-node -- node`

mod context;
mod dex;
mod generator;
mod job;
mod payload;
mod primitives;
mod selectors;

use crate::generator::DexPayloadJobGenerator;
use alloy_primitives::{address, Address};
use reth_chain_state::CanonStateSubscriptions;
use reth_node_api::{NodeTypes, TxTy};
use reth_node_builder::{components::PayloadServiceBuilder, node::FullNodeTypes, BuilderContext};
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_cli::Cli;
use reth_optimism_evm::OpEvmConfig;
use reth_optimism_node::{node::OpAddOns, OpEngineTypes, OpNode};
use reth_optimism_primitives::OpPrimitives;
use reth_optimism_txpool::OpPooledTx;
use reth_payload_builder::{PayloadBuilderHandle, PayloadBuilderService};
use reth_transaction_pool::TransactionPool;

/// DEX predeploy address (same as in op-rbuilder)
pub const DEX_PREDEPLOY_ADDRESS: Address = address!("4200000000000000000000000000000000000042");

/// Custom payload builder service that uses DEX-aware payload generation
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct DexPayloadServiceBuilder;

impl<Node, Pool> PayloadServiceBuilder<Node, Pool, OpEvmConfig> for DexPayloadServiceBuilder
where
    Node: FullNodeTypes<
        Types: NodeTypes<
            Payload = OpEngineTypes,
            ChainSpec = OpChainSpec,
            Primitives = OpPrimitives,
        >,
    >,
    Pool: TransactionPool<Transaction: OpPooledTx<Consensus = TxTy<Node::Types>>> + Unpin + 'static,
{
    async fn spawn_payload_builder_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
        evm_config: OpEvmConfig,
    ) -> eyre::Result<PayloadBuilderHandle<<Node::Types as NodeTypes>::Payload>> {
        tracing::info!("Spawning DEX-aware Optimism payload builder");

        let payload_generator =
            DexPayloadJobGenerator::new(ctx.provider().clone(), pool, evm_config);

        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor()
            .spawn_critical("dex payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

fn main() {
    Cli::parse_args()
        .run(|builder, _| async move {
            let op_node = OpNode::default();
            let handle = builder
                .with_types_and_provider::<OpNode, _>()
                // Use custom DEX-aware payload builder
                .with_components(
                    op_node
                        .components()
                        .payload(DexPayloadServiceBuilder::default()),
                )
                .with_add_ons(OpAddOns::default())
                .launch_with_debug_capabilities()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
