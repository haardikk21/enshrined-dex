//! In-memory orderbook DEX library for trading ETH and ERC-20 style tokens.
//!
//! This library provides an efficient orderbook implementation with:
//! - Limit and market order support
//! - Multi-pair management
//! - Quote generation with automatic multi-hop routing
//! - Configurable fee structure

pub mod config;
pub mod order;
pub mod orderbook;
pub mod pair;
pub mod pool_manager;
pub mod router;
pub mod types;

pub use config::DexConfig;
pub use order::{Order, OrderId, OrderSide, OrderStatus, OrderType};
pub use orderbook::{OrderBook, OrderError};
pub use pair::{Pair, PairId};
pub use pool_manager::{PoolManager, PoolError};
pub use router::{Quote, Route, RouteHop};
pub use types::{Address, Amount, Price, TokenId, U256, ETH_TOKEN};
