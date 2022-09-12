#![feature(error_generic_member_access)]
#![feature(provide_any)]
#![feature(generic_const_exprs)]
#![feature(allocator_api)]

#[macro_use]
extern crate derive_builder;

pub mod config;
#[allow(dead_code)]
pub mod ring;
pub mod util;
pub mod memory;
pub mod device;
pub mod hash;
pub mod ip_frag;

pub mod raw {
    pub use dpdk_sys::*;
}

pub mod eal;

#[cfg(test)]
mod test {
    use crate::config::{DPDKConfigBuilder, CoreConfig};

    #[test]
    pub fn test_dpdk_config_display() {
        let cfg = DPDKConfigBuilder::default()
            .cores(CoreConfig::List(vec![1,2,3,4]))
            .build()
            .unwrap();

        println!("{cfg}");
    }
}
