use dpdk_sys::{rte_eth_rss_reta_entry64};

use crate::memory::allocator::DPDKBox;

pub struct RSSRedirectionTable<const NUM_ENTRIES: usize> {
    table: DPDKBox<[rte_eth_rss_reta_entry64; NUM_ENTRIES]>
}

impl<const NUM_ENTRIES: usize> RSSRedirectionTable<NUM_ENTRIES> {
    pub fn new() -> Self {
        Self {
            // Zeroed out versions of rte_eth_rss_reta_entry64 are valid
            table: unsafe { DPDKBox::new_zeroed() },
        }
    }
}

