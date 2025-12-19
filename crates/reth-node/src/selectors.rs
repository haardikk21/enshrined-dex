//! DEX predeploy contract bindings and selectors.

use alloy_sol_macro::sol;

sol!(
    EnshrinedDEX,
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/out/EnshrinedDEX.sol/EnshrinedDEX.json"
    )
);

sol!(
    DexToken,
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/out/DexToken.sol/DexToken.json"
    )
);

pub mod selectors {
    use super::EnshrinedDEX;
    use alloy_sol_types::SolCall;

    pub const CREATE_PAIR: [u8; 4] = EnshrinedDEX::createPairCall::SELECTOR;
    pub const PLACE_LIMIT_ORDER: [u8; 4] = EnshrinedDEX::placeLimitOrderCall::SELECTOR;
    pub const CANCEL_ORDER: [u8; 4] = EnshrinedDEX::cancelOrderCall::SELECTOR;
    pub const SWAP: [u8; 4] = EnshrinedDEX::swapCall::SELECTOR;
    pub const GET_QUOTE: [u8; 4] = EnshrinedDEX::getQuoteCall::SELECTOR;
}
