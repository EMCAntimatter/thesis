use std::{collections::HashMap, ffi::CString, fmt::Display};

use itertools::Itertools;

use crate::eal::{self, LCoreId};

#[derive(Debug, Clone)]
pub enum CoreConfig {
    Mask(u64),
    List(Vec<LCoreId>),
}

#[derive(Debug, Clone, Copy)]
pub struct PCIAddress {
    pub domain: u16,
    pub bus: u8,
    pub device: u8,   // 5 bits
    pub function: u8, // 3 bits
}

impl Display for PCIAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}:{:02}.{:02}.{:1}",
            self.domain, self.bus, self.device, self.function
        )
    }
}

#[derive(Debug, Clone)]
pub struct VirtualDevice {
    pub driver: String,
    pub id: u64,
    pub options: HashMap<String, String>,
}

impl VirtualDevice {
    pub fn with_driver(driver: impl ToString) -> VirtualDevice {
        Self {
            driver: driver.to_string(),
            id: 0,
            options: Default::default(),
        }
    }
}

impl Display for VirtualDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let options = self
            .options
            .iter()
            .map(|(k, v)| k.to_owned() + "=" + v)
            .fold("".to_string(), |a, b| format!("{a},{b}").to_string());
        write!(f, "{}{}{}", self.driver, self.id, options)
    }
}

#[derive(Debug, Clone)]
pub enum PCIOptions {
    NoPCI,
    PCI {
        blocked_devices: Vec<PCIAddress>,
        allowed_devices: Vec<PCIAddress>,
    },
}

impl Default for PCIOptions {
    fn default() -> Self {
        Self::NoPCI
    }
}

#[derive(Debug, Clone)]
pub enum MultiprocessingProcType {
    Primary,
    Secondary,
    Auto,
}

impl Default for MultiprocessingProcType {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone)]
pub enum IOVAMode {
    PA,
    VA,
}

#[derive(Debug, Builder)]
pub struct DPDKConfig {
    pub cores: CoreConfig,
    pub main_lcore: Option<LCoreId>,
    pub service_core_mask: Option<u64>,
    pub pci_options: PCIOptions,
    pub virtual_devices: Vec<VirtualDevice>,
    pub num_memory_channels: Option<u64>,
}

impl DPDKConfig {
    pub fn apply(self) -> Result<(), eal::RteErrnoValue> {
        let f = format!("{}", &self);
        println!("{}", self);
        let args = f
            .split_ascii_whitespace()
            .map(|arg| CString::new(arg).unwrap())
            .collect_vec();
        let c_args = args.iter().map(|arg| arg.as_ptr() as *mut libc::c_char).collect_vec();
        println!("{}", unsafe { dpdk_sys::per_lcore__rte_errno });
        let ret = unsafe { dpdk_sys::rte_eal_init(c_args.len() as i32, c_args.as_ptr() as *mut _) };

        if ret >= 0 {
            Ok(())
        } else {
            println!("{}", unsafe { dpdk_sys::per_lcore__rte_errno });
            let err = eal::RteErrnoValue::most_recent();
            Err(err)
        }
    }
}

impl Default for DPDKConfig {
    fn default() -> Self {
        Self {
            cores: CoreConfig::Mask(1),
            main_lcore: Default::default(),
            service_core_mask: Default::default(),
            pci_options: Default::default(),
            virtual_devices: Default::default(),
            num_memory_channels: Default::default(),
        }
    }
}

impl Display for DPDKConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut options: Vec<String> = vec![];
        match &self.cores {
            CoreConfig::Mask(mask) => {
                write!(f, "{} {:x} ", "-c", mask)?;
            }
            CoreConfig::List(list) => {
                let cores = list.iter().map(|c| c.to_string()).join(",");
                write!(f, "-l {} ", cores)?;
            }
        }

        if let Some(val) = self.main_lcore {
            write!(f, "--main-lcore {} ", val)?;
        }

        if let Some(service_core_mask) = self.service_core_mask {
            write!(f, "-s {:x} ", service_core_mask)?;
        }

        match &self.pci_options {
            PCIOptions::NoPCI => options.push("--no-pci".to_string()),
            PCIOptions::PCI {
                blocked_devices,
                allowed_devices,
            } => {
                for blocked_device in blocked_devices {
                    write!(f, "{} {} ", "-b", blocked_device)?;
                }

                for allowed_device in allowed_devices {
                    write!(f, "-a {} ", allowed_device)?;
                }
            }
        }

        for virtual_device in &self.virtual_devices {
            write!(f, "--vdevs {} ", virtual_device)?;
        }

        if let Some(memory_channels) = &self.num_memory_channels {
            write!(f, "-n {} ", memory_channels)?
        } else {
            write!(f, "-n 1 ")?
        }

        Ok(())
    }
}
