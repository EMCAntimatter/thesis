// extern crate bindgen;
extern crate core;

use std::{path::{PathBuf}, env};

const HEADERS: [&str; 61] = [
    "rte_alarm.h",
    "rte_arp.h",
    "rte_bitops.h",
    "rte_bitrate.h",
    "rte_bpf_ethdev.h",
    "rte_branch_prediction.h",
    "rte_bus_pci.h",
    "rte_byteorder.h",
    "rte_common.h",
    "rte_cycles.h",
    "rte_debug.h",
    "rte_devargs.h",
    "rte_dev.h",
    "rte_eal.h",
    "rte_errno.h",
    "rte_eth_bond_8023ad.h",
    "rte_eth_bond.h",
    "rte_ethdev.h",
    "rte_ether.h",
    "rte_flow.h",
    "rte_geneve.h",
    "rte_gre.h",
    "rte_gro.h",
    "rte_gso.h",
    "rte_gtp.h",
    "rte_hexdump.h",
    "rte_icmp.h",
    "rte_interrupts.h",
    "rte_ip.h",
    "rte_latencystats.h",
    "rte_launch.h",
    "rte_lcore.h",
    "rte_log.h",
    "rte_malloc.h",
    "rte_mbuf_dyn.h",
    "rte_mbuf.h",
    "rte_mbuf_pool_ops.h",
    "rte_memcpy.h",
    "rte_memory.h",
    "rte_mempool.h",
    "rte_memzone.h",
    "rte_metrics.h",
    "rte_mpls.h",
    "rte_mtr.h",
    "rte_net.h",
    "rte_pci.h",
    "rte_pdump.h",
    "rte_per_lcore.h",
    "rte_pmd_bnxt.h",
    "rte_pmd_dpaa.h",
    "rte_pmd_i40e.h",
    "rte_pmd_ixgbe.h",
    "rte_prefetch.h",
    "rte_ring.h",
    "rte_sctp.h",
    "rte_string_fns.h",
    "rte_tcp.h",
    "rte_tm.h",
    "rte_udp.h",
    "rte_vect.h",
    "rte_vxlan.h",
];


trait BindgenBuilderExtensions {
    fn add_replaced_types(self) -> Self;

    fn add_headers(self, include_path: &mut PathBuf) -> Self;
}

impl BindgenBuilderExtensions for bindgen::Builder {
    fn add_replaced_types(self) -> Self {
        self.blocklist_type("rte_l2tpv2_common_hdr")
            .blocklist_type("rte_ether_hdr")
            .blocklist_type("rte_ether_addr")
            .blocklist_type("marker_header")
            .blocklist_function(".*scanf.*")
            .blocklist_function("strerror_r")
            .blocklist_item("per_lcore__lcore_id")
    }

    fn add_headers(self, include_path: &mut PathBuf) -> Self {
        let mut s = self;
        for header in HEADERS {
            include_path.push(header);
            s = s.header(include_path.to_str().unwrap());
            include_path.pop();
        }
        return s;
    }
}

fn link_with_dpdk() {
    std::env::set_var("PKG_CONFIG_PATH", "/usr/local/lib/pkgconfig");
    std::env::set_var("PKG_CONFIG_ALL_STATIC", "1");

    let mut libdpdk = pkg_config::Config::new()
        .arg("--with-path=/usr/local/lib/pkgconfig")
        .statik(true)
        .probe("libdpdk")
        .unwrap();

    println!("cargo:rustc-link-search=native=/usr/local/lib");
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    // println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        // .clang_args(libdpdk.ld_args.iter().flatten())
        .generate_comments(true)
        .detect_include_paths(true)
        .derive_debug(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // .layout_tests(false)
        .add_replaced_types()
        .generate_inline_functions(true)
        .enable_function_attribute_detection()
        .ctypes_prefix("libc")
        .add_headers(&mut libdpdk.include_paths[0])
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");
    
    
    // // Write the bindings to the $OUT_DIR/bindings.rs file.
    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // bindings
    //     .write_to_file(out_path.join("bindings.rs"))
    //     .expect("Couldn't write bindings!");
}

fn use_system_deps() {
    system_deps::Config::new().probe().unwrap();
}

fn main() {
    // println!("cargo:rustc-link-search={}/dpdk-22.03/build/lib", env!("CARGO_MANIFEST_DIR"));
    // println!("cargo:rustc-link-lib=dpdk-rs");
    // println!("cargo:rustc-env=dpdk-rs={}/dpdk-22.03/build/lib", env!("CARGO_MANIFEST_DIR"))
    // link_with_dpdk();
    
    println!("cargo:rerun-if-changed={}/dpdk-22.03/build/lib/bindings.rs", env!("CARGO_MANIFEST_DIR"));
    use_system_deps();
}