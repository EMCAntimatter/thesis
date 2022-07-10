use std::mem::MaybeUninit;

use crate::{rte_eth_conf, rte_eth_rxmode, rte_eth_txmode};

impl Default for rte_eth_rxmode {
    fn default() -> Self {
        // This struct is supposed to zero out all unset options
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}

impl Default for rte_eth_txmode {
    fn default() -> Self {
        // This struct is supposed to zero out all unset options
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}

impl Default for rte_eth_conf {
    fn default() -> Self {
        // This struct is supposed to zero out all unset options
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}
