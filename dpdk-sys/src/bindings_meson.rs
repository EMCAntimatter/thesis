#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::replacements::*;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/dpdk-22.03/build/lib/rust_bindgen/bindings.rs"));