use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::ptr::null_mut;
use libc::{c_int, c_void};
use dpdk_sys::{per_lcore__lcore_id, rte_eal_cleanup, rte_eal_mp_wait_lcore};
use dpdk_sys::rte_lcore_foreach_worker;

unsafe extern "C" fn lcore_hello(_unused: *mut c_void) -> c_int {
    println!("hello from lcore {}", per_lcore__lcore_id);
    return 0;
}

fn main() {
    unsafe {
        let args: Vec<&[u8]> = vec![b"-l\0", b"0-3\0", b"-n\0", b"4\0"];
        let mut argv: Vec<*mut i8> = args.into_iter()
            .map(|arg| arg.as_ptr() as *mut i8)
            .collect();
        argv.push(null_mut());
        print!("{:#?}", &argv);
        let ret = dpdk_sys::rte_eal_init(argv.len() as i32, argv.as_mut_ptr());
        if ret < 0 {
            panic!("Cannot init EAL");
        }

        panic!("Working");

        rte_lcore_foreach_worker(|lcore_id| {
            dpdk_sys::rte_eal_remote_launch(Some(lcore_hello), null_mut(), lcore_id);
        });

        lcore_hello(null_mut());

        rte_eal_mp_wait_lcore();

        rte_eal_cleanup();
    }
}
