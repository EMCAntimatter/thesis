use dpdk_sys::{rte_hash, rte_hash_parameters, RTE_HASH_ENTRIES_MAX, RTE_HASH_NAMESIZE, RTE_HASH_LOOKUP_BULK_MAX};
use libc::c_void;

use std::marker::PhantomData;

use crate::util::cast_str_to_i8_ptr;

pub const MAX_TABLE_SIZE: u32 = RTE_HASH_ENTRIES_MAX;
pub const MAX_KEY_LEN: u32 = RTE_HASH_NAMESIZE;
pub const MAX_BULK_LOOKUP_KEYS: u32 = RTE_HASH_LOOKUP_BULK_MAX;

pub struct DPDKHashTable<'table> {
    inner: *mut rte_hash,
    phantom: PhantomData<&'table mut rte_hash>
}

impl<'table> DPDKHashTable<'table> {
    pub fn new(
        name: &'static str,
        entries: u32,
        function: extern "C" fn(*const c_void, u32, u32) -> u32,
        hash_key_len: u32,
        initial_value: u32,
        numa_socket_id: i32,
        extra_flag: u8,
    ) {
        let table_params = rte_hash_parameters {
            name: cast_str_to_i8_ptr(name),
            entries,
            reserved: 0,
            key_len: hash_key_len,
            hash_func: Some(function),
            hash_func_init_val: initial_value,
            socket_id: numa_socket_id,
            extra_flag,
        };
    }
}
