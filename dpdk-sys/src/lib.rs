#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![feature(thread_local)]

// pub(crate) mod bindings;
pub(crate) mod bindings_meson;
pub(crate) mod impls;
pub(crate) mod replacements;

// pub use bindings::*;
pub use bindings_meson::*;
pub use impls::*;
pub use replacements::*;
