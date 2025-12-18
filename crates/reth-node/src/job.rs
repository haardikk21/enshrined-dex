//! DEX-aware payload job implementation.

use crate::payload::DexOpPayloadBuilder;
use futures_util::Future;
use reth_basic_payload_builder::{BuildArguments, BuildOutcome, PayloadBuilder, PayloadConfig};
use reth_node_api::{PayloadBuilderAttributes, PayloadKind};
use reth_optimism_node::{OpBuiltPayload, OpPayloadBuilderAttributes};
use reth_optimism_primitives::OpTransactionSigned;
use reth_payload_builder::{KeepPayloadJobAlive, PayloadBuilderError, PayloadJob};
use reth_provider::{ChainSpecProvider, StateProviderFactory};
use reth_transaction_pool::TransactionPool;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tracing::{debug, info};

/// A PayloadJob that uses the DEX-aware payload builder.
pub struct DexPayloadJob<Pool, Client>
where
    Pool: TransactionPool<Transaction: reth_optimism_txpool::OpPooledTx>,
    Client: StateProviderFactory,
{
    config: PayloadConfig<OpPayloadBuilderAttributes<OpTransactionSigned>>,
    builder: DexOpPayloadBuilder<Pool, Client>,
    best_payload: parking_lot::RwLock<Option<OpBuiltPayload>>,
    building_started: std::sync::atomic::AtomicBool,
}

impl<Pool, Client> DexPayloadJob<Pool, Client>
where
    Pool: TransactionPool<Transaction: reth_optimism_txpool::OpPooledTx>,
    Client: StateProviderFactory,
{
    pub fn new(
        config: PayloadConfig<OpPayloadBuilderAttributes<OpTransactionSigned>>,
        builder: DexOpPayloadBuilder<Pool, Client>,
    ) -> Self {
        Self {
            config,
            builder,
            best_payload: parking_lot::RwLock::new(None),
            building_started: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl<Pool, Client> PayloadJob for DexPayloadJob<Pool, Client>
where
    Pool: TransactionPool<
            Transaction: reth_optimism_txpool::OpPooledTx<
                Consensus = op_alloy_consensus::OpTxEnvelope,
            >,
        > + Unpin
        + 'static,
    Client: StateProviderFactory
        + ChainSpecProvider<ChainSpec = reth_optimism_chainspec::OpChainSpec>
        + Clone
        + Unpin
        + 'static,
{
    type PayloadAttributes = OpPayloadBuilderAttributes<OpTransactionSigned>;
    type ResolvePayloadFuture =
        futures_util::future::Ready<Result<Self::BuiltPayload, PayloadBuilderError>>;
    type BuiltPayload = OpBuiltPayload;

    fn best_payload(&self) -> Result<Self::BuiltPayload, PayloadBuilderError> {
        if let Some(payload) = self.best_payload.read().clone() {
            return Ok(payload);
        }

        let args = BuildArguments {
            config: self.config.clone(),
            cached_reads: Default::default(),
            cancel: Default::default(),
            best_payload: None,
        };

        match self.builder.try_build(args)? {
            BuildOutcome::Better { payload, .. } | BuildOutcome::Freeze(payload) => {
                *self.best_payload.write() = Some(payload.clone());
                Ok(payload)
            }
            BuildOutcome::Aborted { fees, .. } => {
                debug!(target: "payload_builder", ?fees, "Build aborted");
                Err(PayloadBuilderError::MissingPayload)
            }
            BuildOutcome::Cancelled => Err(PayloadBuilderError::MissingPayload),
        }
    }

    fn payload_attributes(&self) -> Result<Self::PayloadAttributes, PayloadBuilderError> {
        Ok(self.config.attributes.clone())
    }

    fn payload_timestamp(&self) -> Result<u64, PayloadBuilderError> {
        Ok(self.config.attributes.timestamp())
    }

    fn resolve_kind(
        &mut self,
        kind: PayloadKind,
    ) -> (Self::ResolvePayloadFuture, KeepPayloadJobAlive) {
        info!(target: "payload_builder", ?kind, "Resolving DEX payload job");
        (
            futures_util::future::ready(self.best_payload()),
            KeepPayloadJobAlive::No,
        )
    }
}

impl<Pool, Client> Future for DexPayloadJob<Pool, Client>
where
    Pool: TransactionPool<
            Transaction: reth_optimism_txpool::OpPooledTx<
                Consensus = op_alloy_consensus::OpTxEnvelope,
            >,
        > + Unpin
        + 'static,
    Client: StateProviderFactory
        + ChainSpecProvider<ChainSpec = reth_optimism_chainspec::OpChainSpec>
        + Clone
        + Unpin
        + 'static,
{
    type Output = Result<(), PayloadBuilderError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if !this
            .building_started
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            let args = BuildArguments {
                config: this.config.clone(),
                cached_reads: Default::default(),
                cancel: Default::default(),
                best_payload: None,
            };

            if let Ok(outcome) = this.builder.try_build(args) {
                if let BuildOutcome::Better { payload, .. } | BuildOutcome::Freeze(payload) =
                    outcome
                {
                    debug!(target: "payload_builder", "Built initial DEX payload");
                    *this.best_payload.write() = Some(payload);
                }
            }
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
