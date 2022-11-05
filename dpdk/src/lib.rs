#![feature(error_generic_member_access)]
#![feature(provide_any)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(allocator_api)]
#![feature(new_uninit)]
#![feature(iter_array_chunks)]
#![feature(generic_associated_types)]
#![feature(test)]

extern crate test;

#[macro_use]
extern crate derive_builder;

pub mod config;
pub mod device;
pub mod ip_frag;
pub mod memory;
#[allow(dead_code)]
pub mod ring;
pub mod rss;
pub mod util;

pub mod raw {
    pub use dpdk_sys::*;
}

pub mod eal;
