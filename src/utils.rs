use alloy::primitives::U256;

pub fn wei_to_eth(wei: U256) -> f64 {
    wei.to::<u128>() as f64 / 1e18
}
