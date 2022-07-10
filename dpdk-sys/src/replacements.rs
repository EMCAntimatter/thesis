use crate::{bindings_meson::{
    rte_get_next_lcore,
    RTE_MAX_LCORE,
    __BindgenBitfieldUnit,
}, rte_mbuf};

extern "C" {
    #[thread_local]
    pub static mut per_lcore__lcore_id: libc::c_int;

    #[thread_local]
    pub static mut per_lcore__rte_errno: libc::c_int;

    #[thread_local]
    pub static mut per_lcore__thread_id: libc::c_int;
}


#[macro_export]
macro_rules! RTE_LCORE_FOREACH {
    ($e:expr) => {
        unsafe {
            let mut lcore = rte_get_next_lcore(u32::MAX, 1, 0);
            while lcore < RTE_MAX_LCORE {
                $e
                lcore = rte_get_next_lcore(lcore, 1, 0);
            }
        }
    };
}

#[inline]
pub fn rte_lcore_foreach_worker(mut f: impl FnMut(u32)) {
    unsafe {
        let mut i = rte_get_next_lcore(u32::MAX, 1, 0);
        while i < RTE_MAX_LCORE {
            f(i);
            i = rte_get_next_lcore(i, 1, 0);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct rte_l2tpv2_common_hdr {
    _bitfield_1: __BindgenBitfieldUnit<[u8; 2]>,
}

impl rte_l2tpv2_common_hdr {
    #[inline]
    pub fn ver(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 4u8) as u16) }
    }
    #[inline]
    pub fn set_ver(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 4u8, val as u64)
        }
    }
    #[inline]
    pub fn res3(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(4usize, 4u8) as u16) }
    }
    #[inline]
    pub fn set_res3(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(4usize, 4u8, val as u64)
        }
    }
    #[inline]
    pub fn p(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(8usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_p(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(8usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn o(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(9usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_o(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(9usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn res2(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(10usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_res2(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(10usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn s(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(11usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_s(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(11usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn res1(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(12usize, 2u8) as u16) }
    }
    #[inline]
    pub fn set_res1(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(12usize, 2u8, val as u64)
        }
    }
    #[inline]
    pub fn l(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(14usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_l(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(14usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn t(&self) -> u16 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(15usize, 1u8) as u16) }
    }
    #[inline]
    pub fn set_t(&mut self, val: u16) {
        unsafe {
            let val: u16 = ::std::mem::transmute(val);
            self._bitfield_1.set(15usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        ver: u16,
        res3: u16,
        p: u16,
        o: u16,
        res2: u16,
        s: u16,
        res1: u16,
        l: u16,
        t: u16,
    ) -> __BindgenBitfieldUnit<[u8; 2usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 2usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 4u8, {
            let ver: u16 = unsafe { ::std::mem::transmute(ver) };
            ver as u64
        });
        __bindgen_bitfield_unit.set(4usize, 4u8, {
            let res3: u16 = unsafe { ::std::mem::transmute(res3) };
            res3 as u64
        });
        __bindgen_bitfield_unit.set(8usize, 1u8, {
            let p: u16 = unsafe { ::std::mem::transmute(p) };
            p as u64
        });
        __bindgen_bitfield_unit.set(9usize, 1u8, {
            let o: u16 = unsafe { ::std::mem::transmute(o) };
            o as u64
        });
        __bindgen_bitfield_unit.set(10usize, 1u8, {
            let res2: u16 = unsafe { ::std::mem::transmute(res2) };
            res2 as u64
        });
        __bindgen_bitfield_unit.set(11usize, 1u8, {
            let s: u16 = unsafe { ::std::mem::transmute(s) };
            s as u64
        });
        __bindgen_bitfield_unit.set(12usize, 2u8, {
            let res1: u16 = unsafe { ::std::mem::transmute(res1) };
            res1 as u64
        });
        __bindgen_bitfield_unit.set(14usize, 1u8, {
            let l: u16 = unsafe { ::std::mem::transmute(l) };
            l as u64
        });
        __bindgen_bitfield_unit.set(15usize, 1u8, {
            let t: u16 = unsafe { ::std::mem::transmute(t) };
            t as u64
        });
        __bindgen_bitfield_unit
    }
}

// #[repr(C, align(2))]
// pub struct marker_header {
//     pub eth_hdr: rte_ether_hdr,
//     pub marker: crate::marker,
// }

/// Ethernet header: Contains the destination address, source address
///  and frame type.
#[repr(C)]
pub struct rte_ether_hdr {
    /// Destination address.
    pub dst_addr: rte_ether_addr,
    /// Source address.
    pub src_addr: rte_ether_addr,
    /// Frame type, Big Endian
    pub ether_type: u16,
}

///  Ethernet address:
///  A universally administered address is uniquely assigned to a device by its
///  manufacturer. The first three octets (in transmission order) contain the
///  Organizationally Unique Identifier (OUI). The following three (MAC-48 and
///  EUI-48) octets are assigned by that organization with the only constraint
///  of uniqueness.
///  A locally administered address is assigned to a device by a network
///  administrator and does not contain OUIs.
///  See http://standards.ieee.org/regauth/groupmac/tutorial.html
#[repr(C)]
pub struct rte_ether_addr {
    /// Addr bytes in tx order
    pub addr_bytes: [u8; crate::RTE_ETHER_ADDR_LEN as usize],
}

extern "C" {
    /// Retrieve a burst of input packets from a receive queue of an Ethernet
    /// device. The retrieved packets are stored in *rte_mbuf* structures whose
    /// pointers are supplied in the *rx_pkts* array.
    ///
    /// The rte_eth_rx_burst() function loops, parsing the Rx ring of the
    /// receive queue, up to *nb_pkts* packets, and for each completed Rx
    /// descriptor in the ring, it performs the following operations:
    ///
    /// - Initialize the *rte_mbuf* data structure associated with the
    ///   Rx descriptor according to the information provided by the NIC into
    ///   that Rx descriptor.
    ///
    /// - Store the *rte_mbuf* data structure into the next entry of the
    ///   *rx_pkts* array.
    ///
    /// - Replenish the Rx descriptor with a new *rte_mbuf* buffer
    ///   allocated from the memory pool associated with the receive queue at
    ///   initialization time.
    ///
    /// When retrieving an input packet that was scattered by the controller
    /// into multiple receive descriptors, the rte_eth_rx_burst() function
    /// appends the associated *rte_mbuf* buffers to the first buffer of the
    /// packet.
    ///
    /// The rte_eth_rx_burst() function returns the number of packets
    /// actually retrieved, which is the number of *rte_mbuf* data structures
    /// effectively supplied into the *rx_pkts* array.
    /// A return value equal to *nb_pkts* indicates that the Rx queue contained
    /// at least *rx_pkts* packets, and this is likely to signify that other
    /// received packets remain in the input queue. Applications implementing
    /// a "retrieve as much received packets as possible" policy can check this
    /// specific case and keep invoking the rte_eth_rx_burst() function until
    /// a value less than *nb_pkts* is returned.
    ///
    /// This receive method has the following advantages:
    ///
    /// - It allows a run-to-completion network stack engine to retrieve and
    ///   to immediately process received packets in a fast burst-oriented
    ///   approach, avoiding the overhead of unnecessary intermediate packet
    ///   queue/dequeue operations.
    ///
    /// - Conversely, it also allows an asynchronous-oriented processing
    ///   method to retrieve bursts of received packets and to immediately
    ///   queue them for further parallel processing by another logical core,
    ///   for instance. However, instead of having received packets being
    ///   individually queued by the driver, this approach allows the caller
    ///   of the rte_eth_rx_burst() function to queue a burst of retrieved
    ///   packets at a time and therefore dramatically reduce the cost of
    ///   enqueue/dequeue operations per packet.
    ///
    /// - It allows the rte_eth_rx_burst() function of the driver to take
    ///   advantage of burst-oriented hardware features (CPU cache,
    ///   prefetch instructions, and so on) to minimize the number of CPU
    ///   cycles per packet.
    ///
    /// To summarize, the proposed receive API enables many
    /// burst-oriented optimizations in both synchronous and asynchronous
    /// packet processing environments with no overhead in both cases.
    ///
    /// @note
    ///   Some drivers using vector instructions require that *nb_pkts* is
    ///   divisible by 4 or 8, depending on the driver implementation.
    ///
    /// The rte_eth_rx_burst() function does not provide any error
    /// notification to avoid the corresponding overhead. As a hint, the
    /// upper-level application might check the status of the device link once
    /// being systematically returned a 0 value for a given number of tries.
    ///
    /// * `port_id` - The port identifier of the Ethernet device.
    /// * `queue_id` - The index of the receive queue from which to retrieve input packets.
    ///   The value must be in the range \[0, nb_rx_queue - 1\] previously supplied
    ///   to `rte_eth_dev_configure`.
    /// * `rx_pkts` - The address of an array of pointers to `rte_mbuf` structures that
    ///   must be large enough to store *nb_pkts* pointers in it.
    /// * `nb_pkts` - The maximum number of packets to retrieve. The value must be divisible by 8 in order to work with any driver.
    /// 
    /// @return
    ///   The number of packets actually retrieved, which is the number
    ///   of pointers to *rte_mbuf* structures effectively supplied to the
    ///   *rx_pkts* array.
    ////
    pub fn rte_eth_rx_burst(port_id: u16, queue_id: u16, rx_pkts: *mut*mut rte_mbuf, nb_pkts: u16) -> u16;
}