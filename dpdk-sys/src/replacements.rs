use crate::bindings_meson::{
    rte_get_next_lcore,
    RTE_MAX_LCORE,
    __BindgenBitfieldUnit,
};

extern "C" {
    #[thread_local]
    pub static mut per_lcore__lcore_id: libc::c_int;
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