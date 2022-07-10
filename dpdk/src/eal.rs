use std::{backtrace::Backtrace, process::exit};

use dpdk_sys::rte_exit;
pub use dpdk_sys::{per_lcore__lcore_id, per_lcore__rte_errno, per_lcore__thread_id};

#[repr(u32)]
#[derive(num_derive::FromPrimitive, Debug)]
pub enum RteErrnoValue {
    EPERM = dpdk_sys::EPERM,
    ENOENT = dpdk_sys::ENOENT,
    ESRCH = dpdk_sys::ESRCH,
    EINTR = dpdk_sys::EINTR,
    EIO = dpdk_sys::EIO,
    ENXIO = dpdk_sys::ENXIO,
    E2BIG = dpdk_sys::E2BIG,
    ENOEXEC = dpdk_sys::ENOEXEC,
    EBADF = dpdk_sys::EBADF,
    ECHILD = dpdk_sys::ECHILD,
    EAGAIN = dpdk_sys::EAGAIN,
    ENOMEM = dpdk_sys::ENOMEM,
    EACCES = dpdk_sys::EACCES,
    EFAULT = dpdk_sys::EFAULT,
    ENOTBLK = dpdk_sys::ENOTBLK,
    EBUSY = dpdk_sys::EBUSY,
    EEXIST = dpdk_sys::EEXIST,
    EXDEV = dpdk_sys::EXDEV,
    ENODEV = dpdk_sys::ENODEV,
    ENOTDIR = dpdk_sys::ENOTDIR,
    EISDIR = dpdk_sys::EISDIR,
    EINVAL = dpdk_sys::EINVAL,
    ENFILE = dpdk_sys::ENFILE,
    EMFILE = dpdk_sys::EMFILE,
    ENOTTY = dpdk_sys::ENOTTY,
    ETXTBSY = dpdk_sys::ETXTBSY,
    EFBIG = dpdk_sys::EFBIG,
    ENOSPC = dpdk_sys::ENOSPC,
    ESPIPE = dpdk_sys::ESPIPE,
    EROFS = dpdk_sys::EROFS,
    EMLINK = dpdk_sys::EMLINK,
    EPIPE = dpdk_sys::EPIPE,
    EDOM = dpdk_sys::EDOM,
    ERANGE = dpdk_sys::ERANGE,
    EDEADLK = dpdk_sys::EDEADLK,
    ENAMETOOLONG = dpdk_sys::ENAMETOOLONG,
    ENOLCK = dpdk_sys::ENOLCK,
    ENOSYS = dpdk_sys::ENOSYS,
    ENOTEMPTY = dpdk_sys::ENOTEMPTY,
    ELOOP = dpdk_sys::ELOOP,
    ENOMSG = dpdk_sys::ENOMSG,
    EIDRM = dpdk_sys::EIDRM,
    ECHRNG = dpdk_sys::ECHRNG,
    EL2NSYNC = dpdk_sys::EL2NSYNC,
    EL3HLT = dpdk_sys::EL3HLT,
    EL3RST = dpdk_sys::EL3RST,
    ELNRNG = dpdk_sys::ELNRNG,
    EUNATCH = dpdk_sys::EUNATCH,
    ENOCSI = dpdk_sys::ENOCSI,
    EL2HLT = dpdk_sys::EL2HLT,
    EBADE = dpdk_sys::EBADE,
    EBADR = dpdk_sys::EBADR,
    EXFULL = dpdk_sys::EXFULL,
    ENOANO = dpdk_sys::ENOANO,
    EBADRQC = dpdk_sys::EBADRQC,
    EBADSLT = dpdk_sys::EBADSLT,
    EBFONT = dpdk_sys::EBFONT,
    ENOSTR = dpdk_sys::ENOSTR,
    ENODATA = dpdk_sys::ENODATA,
    ETIME = dpdk_sys::ETIME,
    ENOSR = dpdk_sys::ENOSR,
    ENONET = dpdk_sys::ENONET,
    ENOPKG = dpdk_sys::ENOPKG,
    EREMOTE = dpdk_sys::EREMOTE,
    ENOLINK = dpdk_sys::ENOLINK,
    EADV = dpdk_sys::EADV,
    ESRMNT = dpdk_sys::ESRMNT,
    ECOMM = dpdk_sys::ECOMM,
    EPROTO = dpdk_sys::EPROTO,
    EMULTIHOP = dpdk_sys::EMULTIHOP,
    EDOTDOT = dpdk_sys::EDOTDOT,
    EBADMSG = dpdk_sys::EBADMSG,
    EOVERFLOW = dpdk_sys::EOVERFLOW,
    ENOTUNIQ = dpdk_sys::ENOTUNIQ,
    EBADFD = dpdk_sys::EBADFD,
    EREMCHG = dpdk_sys::EREMCHG,
    ELIBACC = dpdk_sys::ELIBACC,
    ELIBBAD = dpdk_sys::ELIBBAD,
    ELIBSCN = dpdk_sys::ELIBSCN,
    ELIBMAX = dpdk_sys::ELIBMAX,
    ELIBEXEC = dpdk_sys::ELIBEXEC,
    EILSEQ = dpdk_sys::EILSEQ,
    ERESTART = dpdk_sys::ERESTART,
    ESTRPIPE = dpdk_sys::ESTRPIPE,
    EUSERS = dpdk_sys::EUSERS,
    ENOTSOCK = dpdk_sys::ENOTSOCK,
    EDESTADDRREQ = dpdk_sys::EDESTADDRREQ,
    EMSGSIZE = dpdk_sys::EMSGSIZE,
    EPROTOTYPE = dpdk_sys::EPROTOTYPE,
    ENOPROTOOPT = dpdk_sys::ENOPROTOOPT,
    EPROTONOSUPPORT = dpdk_sys::EPROTONOSUPPORT,
    ESOCKTNOSUPPORT = dpdk_sys::ESOCKTNOSUPPORT,
    EOPNOTSUPP = dpdk_sys::EOPNOTSUPP,
    EPFNOSUPPORT = dpdk_sys::EPFNOSUPPORT,
    EAFNOSUPPORT = dpdk_sys::EAFNOSUPPORT,
    EADDRINUSE = dpdk_sys::EADDRINUSE,
    EADDRNOTAVAIL = dpdk_sys::EADDRNOTAVAIL,
    ENETDOWN = dpdk_sys::ENETDOWN,
    ENETUNREACH = dpdk_sys::ENETUNREACH,
    ENETRESET = dpdk_sys::ENETRESET,
    ECONNABORTED = dpdk_sys::ECONNABORTED,
    ECONNRESET = dpdk_sys::ECONNRESET,
    ENOBUFS = dpdk_sys::ENOBUFS,
    EISCONN = dpdk_sys::EISCONN,
    ENOTCONN = dpdk_sys::ENOTCONN,
    ESHUTDOWN = dpdk_sys::ESHUTDOWN,
    ETOOMANYREFS = dpdk_sys::ETOOMANYREFS,
    ETIMEDOUT = dpdk_sys::ETIMEDOUT,
    ECONNREFUSED = dpdk_sys::ECONNREFUSED,
    EHOSTDOWN = dpdk_sys::EHOSTDOWN,
    EHOSTUNREACH = dpdk_sys::EHOSTUNREACH,
    EALREADY = dpdk_sys::EALREADY,
    EINPROGRESS = dpdk_sys::EINPROGRESS,
    ESTALE = dpdk_sys::ESTALE,
    EUCLEAN = dpdk_sys::EUCLEAN,
    ENOTNAM = dpdk_sys::ENOTNAM,
    ENAVAIL = dpdk_sys::ENAVAIL,
    EISNAM = dpdk_sys::EISNAM,
    EREMOTEIO = dpdk_sys::EREMOTEIO,
    EDQUOT = dpdk_sys::EDQUOT,
    ENOMEDIUM = dpdk_sys::ENOMEDIUM,
    EMEDIUMTYPE = dpdk_sys::EMEDIUMTYPE,
    ECANCELED = dpdk_sys::ECANCELED,
    ENOKEY = dpdk_sys::ENOKEY,
    EKEYEXPIRED = dpdk_sys::EKEYEXPIRED,
    EKEYREVOKED = dpdk_sys::EKEYREVOKED,
    EKEYREJECTED = dpdk_sys::EKEYREJECTED,
    EOWNERDEAD = dpdk_sys::EOWNERDEAD,
    ENOTRECOVERABLE = dpdk_sys::ENOTRECOVERABLE,
    ERFKILL = dpdk_sys::ERFKILL,
    EHWPOISON = dpdk_sys::EHWPOISON,
    RteMinErrno = dpdk_sys::RTE_MIN_ERRNO as u32,
    ERteSecondary = dpdk_sys::E_RTE_SECONDARY as u32,
    ERteNoConfig = dpdk_sys::E_RTE_NO_CONFIG as u32,
    RteMaxErrno = dpdk_sys::RTE_MAX_ERRNO as u32,
}

impl RteErrnoValue {
    pub fn most_recent() -> Self {
        let errno = unsafe { per_lcore__rte_errno } as u32;
        num::FromPrimitive::from_u32(errno).unwrap()
    }

    pub fn clear() {
        unsafe {
            per_lcore__rte_errno = 0;
        }
    }
}

pub type LCoreId = libc::c_int;

pub mod bus {
    use std::ffi::CStr;

    use dpdk_sys::rte_bus;
    use tailq_iterator::TailQIterator;

    #[derive(TailQIterator)]
    pub struct RteBus {
        inner: *mut rte_bus,
    }

    pub fn rte_bus_probe() {
        unsafe {
            dpdk_sys::rte_bus_probe();
        }
    }

    // rte_bus_dump

    extern "C" {
        static mut rte_bus_list: dpdk_sys::rte_bus_list;
    }

    pub fn rte_bus_iter() -> impl Iterator<Item = RteBus> {
        let ptr = unsafe { rte_bus_list.tqh_first };
        RteBusTailQIterator {
            current: Option::from(RteBus { inner: ptr }),
        }
    }

    #[derive(TailQIterator)]
    pub struct RteDevice {
        inner: *mut dpdk_sys::rte_device,
    }

    pub fn rte_bus_find_by_device(dev: &RteDevice) -> RteBus {
        let bus = unsafe { dpdk_sys::rte_bus_find_by_device(dev.inner) };
        RteBus { inner: bus }
    }

    pub fn rte_bus_find_by_name(name: &CStr) -> RteBus {
        let bus = unsafe { dpdk_sys::rte_bus_find_by_name(name.as_ptr()) };
        RteBus { inner: bus }
    }

    pub fn rte_bus_get_iommu_class() -> dpdk_sys::rte_iova_mode {
        unsafe { dpdk_sys::rte_bus_get_iommu_class() }
    }

    pub fn rte_bus_register(bus: &mut RteBus) {
        unsafe {
            dpdk_sys::rte_bus_register(bus.inner);
        }
    }

    pub fn rte_bus_scan() -> Result<(), i32> {
        unsafe {
            let ret = dpdk_sys::rte_bus_scan();
            if ret == 0 {
                Ok(())
            } else {
                Err(ret)
            }
        }
    }

    pub fn rte_bus_unregister(bus: &mut RteBus) {
        unsafe {
            dpdk_sys::rte_bus_unregister(bus.inner);
        }
    }
}

#[inline]
pub fn current_lcore_id() -> i32 {
    unsafe { per_lcore__lcore_id }
}

pub fn dpdk_exit(exit_code: i32, message: &str) -> ! {
    let mut string = message.to_string();
    string.push('\0');
    unsafe { rte_exit(exit_code, string.as_bytes().as_ptr() as *const i8) };
    exit(exit_code)
}

// rte_calloc,
// rte_calloc_socket,
// rte_class_find,
// rte_class_find_by_name,
// rte_class_register,
// rte_class_unregister,
// rte_cpu_get_flag_enabled,
// rte_cpu_get_flag_name,
// rte_ctrl_thread_create,
// rte_delay_us,
// rte_delay_us_block,
// rte_delay_us_callback_register,
// rte_delay_us_sleep,
// rte_dev_dma_map,
// rte_dev_dma_unmap,
// rte_dev_event_callback_process,
// rte_dev_event_callback_register,
// rte_dev_event_callback_unregister,
// rte_dev_event_monitor_start,
// rte_dev_event_monitor_stop,
// rte_dev_hotplug_handle_disable,
// rte_dev_hotplug_handle_enable,
// rte_dev_is_probed,
// rte_dev_iterator_init,
// rte_dev_iterator_next,
// rte_dev_probe,
// rte_dev_remove,
// rte_devargs_add,
// rte_devargs_dump,
// rte_devargs_insert,
// rte_devargs_next,
// rte_devargs_parse,
// rte_devargs_parsef,
// rte_devargs_remove,
// rte_devargs_reset,
// rte_devargs_type_count,
// rte_dump_physmem_layout,
// rte_dump_stack,
// rte_dump_tailq,
// rte_eal_alarm_cancel,
// rte_eal_alarm_set,
// rte_eal_cleanup,
// rte_eal_get_baseaddr,
// rte_eal_get_lcore_state,
// rte_eal_get_physmem_size,
// rte_eal_get_runtime_dir,
// rte_eal_has_hugepages,
// rte_eal_has_pci,
// rte_eal_hotplug_add,
// rte_eal_hotplug_remove,
// rte_eal_init,
// rte_eal_iova_mode,
// rte_eal_lcore_role,
// rte_eal_mbuf_user_pool_ops,
// rte_eal_mp_remote_launch,
// rte_eal_mp_wait_lcore,
// rte_eal_process_type,
// rte_eal_remote_launch,
// rte_eal_tailq_lookup,
// rte_eal_tailq_register,
// rte_eal_using_phys_addrs,
// rte_eal_wait_lcore,
// rte_epoll_ctl,
// rte_epoll_wait,
// rte_epoll_wait_interruptible,
// rte_exit,
// rte_extmem_attach,
// rte_extmem_detach,
// rte_extmem_register,
// rte_extmem_unregister,
// rte_fbarray_attach,
// rte_fbarray_destroy,
// rte_fbarray_detach,
// rte_fbarray_dump_metadata,
// rte_fbarray_find_biggest_free,
// rte_fbarray_find_biggest_used,
// rte_fbarray_find_contig_free,
// rte_fbarray_find_contig_used,
// rte_fbarray_find_idx,
// rte_fbarray_find_next_free,
// rte_fbarray_find_next_n_free,
// rte_fbarray_find_next_n_used,
// rte_fbarray_find_next_used,
// rte_fbarray_find_prev_free,
// rte_fbarray_find_prev_n_free,
// rte_fbarray_find_prev_n_used,
// rte_fbarray_find_prev_used,
// rte_fbarray_find_rev_biggest_free,
// rte_fbarray_find_rev_biggest_used,
// rte_fbarray_find_rev_contig_free,
// rte_fbarray_find_rev_contig_used,
// rte_fbarray_get,
// rte_fbarray_init,
// rte_fbarray_is_used,
// rte_fbarray_set_free,
// rte_fbarray_set_used,
// rte_firmware_read,
// rte_free,
// rte_get_main_lcore,
// rte_get_next_lcore,
// rte_get_tsc_hz,
// rte_hexdump,
// rte_hypervisor_get,
// rte_intr_ack,
// rte_intr_allow_others,
// rte_intr_callback_register,
// rte_intr_callback_unregister,
// rte_intr_callback_unregister_pending,
// rte_intr_callback_unregister_sync,
// rte_intr_cap_multiple,
// rte_intr_dev_fd_get,
// rte_intr_dev_fd_set,
// rte_intr_disable,
// rte_intr_dp_is_en,
// rte_intr_efd_counter_size_get,
// rte_intr_efd_counter_size_set,
// rte_intr_efd_disable,
// rte_intr_efd_enable,
// rte_intr_efds_index_get,
// rte_intr_efds_index_set,
// rte_intr_elist_index_get,
// rte_intr_elist_index_set,
// rte_intr_enable,
// rte_intr_event_list_update,
// rte_intr_fd_get,
// rte_intr_fd_set,
// rte_intr_free_epoll_fd,
// rte_intr_instance_alloc,
// rte_intr_instance_dup,
// rte_intr_instance_free,
// rte_intr_instance_windows_handle_get,
// rte_intr_instance_windows_handle_set,
// rte_intr_max_intr_get,
// rte_intr_max_intr_set,
// rte_intr_nb_efd_get,
// rte_intr_nb_efd_set,
// rte_intr_nb_intr_get,
// rte_intr_rx_ctl,
// rte_intr_tls_epfd,
// rte_intr_type_get,
// rte_intr_type_set,
// rte_intr_vec_list_alloc,
// rte_intr_vec_list_free,
// rte_intr_vec_list_index_get,
// rte_intr_vec_list_index_set,
// rte_lcore_callback_register,
// rte_lcore_callback_unregister,
// rte_lcore_count,
// rte_lcore_cpuset,
// rte_lcore_dump,
// rte_lcore_has_role,
// rte_lcore_index,
// rte_lcore_is_enabled,
// rte_lcore_iterate,
// rte_lcore_to_cpu_id,
// rte_lcore_to_socket_id,
// rte_log,
// rte_log_can_log,
// rte_log_cur_msg_loglevel,
// rte_log_cur_msg_logtype,
// rte_log_dump,
// rte_log_get_global_level,
// rte_log_get_level,
// rte_log_get_stream,
// rte_log_list_types,
// rte_log_register,
// rte_log_register_type_and_pick_level,
// rte_log_set_global_level,
// rte_log_set_level,
// rte_log_set_level_pattern,
// rte_log_set_level_regexp,
// rte_malloc,
// rte_malloc_dump_heaps,
// rte_malloc_dump_stats,
// rte_malloc_get_socket_stats,
// rte_malloc_heap_create,
// rte_malloc_heap_destroy,
// rte_malloc_heap_get_socket,
// rte_malloc_heap_memory_add,
// rte_malloc_heap_memory_attach,
// rte_malloc_heap_memory_detach,
// rte_malloc_heap_memory_remove,
// rte_malloc_heap_socket_is_external,
// rte_malloc_set_limit,
// rte_malloc_socket,
// rte_malloc_validate,
// rte_malloc_virt2iova,
// rte_mcfg_get_single_file_segments,
// rte_mcfg_mem_read_lock,
// rte_mcfg_mem_read_unlock,
// rte_mcfg_mem_write_lock,
// rte_mcfg_mem_write_unlock,
// rte_mcfg_mempool_read_lock,
// rte_mcfg_mempool_read_unlock,
// rte_mcfg_mempool_write_lock,
// rte_mcfg_mempool_write_unlock,
// rte_mcfg_tailq_read_lock,
// rte_mcfg_tailq_read_unlock,
// rte_mcfg_tailq_write_lock,
// rte_mcfg_tailq_write_unlock,
// rte_mcfg_timer_lock,
// rte_mcfg_timer_unlock,
// rte_mem_alloc_validator_register,
// rte_mem_alloc_validator_unregister,
// rte_mem_check_dma_mask,
// rte_mem_check_dma_mask_thread_unsafe,
// rte_mem_event_callback_register,
// rte_mem_event_callback_unregister,
// rte_mem_iova2virt,
// rte_mem_lock,
// rte_mem_lock_page,
// rte_mem_map,
// rte_mem_page_size,
// rte_mem_set_dma_mask,
// rte_mem_unmap,
// rte_mem_virt2iova,
// rte_mem_virt2memseg,
// rte_mem_virt2memseg_list,
// rte_mem_virt2phy,
// rte_memdump,
// rte_memory_get_nchannel,
// rte_memory_get_nrank,
// rte_memseg_contig_walk,
// rte_memseg_contig_walk_thread_unsafe,
// rte_memseg_get_fd,
// rte_memseg_get_fd_offset,
// rte_memseg_get_fd_offset_thread_unsafe,
// rte_memseg_get_fd_thread_unsafe,
// rte_memseg_list_walk,
// rte_memseg_list_walk_thread_unsafe,
// rte_memseg_walk,
// rte_memseg_walk_thread_unsafe,
// rte_memzone_dump,
// rte_memzone_free,
// rte_memzone_lookup,
// rte_memzone_reserve,
// rte_memzone_reserve_aligned,
// rte_memzone_reserve_bounded,
// rte_memzone_walk,
// rte_mp_action_register,
// rte_mp_action_unregister,
// rte_mp_disable,
// rte_mp_reply,
// rte_mp_request_async,
// rte_mp_request_sync,
// rte_mp_sendmsg,
// rte_openlog_stream,
// rte_rand,
// rte_rand_max,
// rte_realloc,
// rte_realloc_socket,
// rte_reciprocal_value,
// rte_reciprocal_value_u64,
// rte_rtm_supported,
// rte_service_attr_get,
// rte_service_attr_reset_all,
// rte_service_component_register,
// rte_service_component_runstate_set,
// rte_service_component_unregister,
// rte_service_dump,
// rte_service_finalize,
// rte_service_get_by_name,
// rte_service_get_count,
// rte_service_get_name,
// rte_service_lcore_add,
// rte_service_lcore_attr_get,
// rte_service_lcore_attr_reset_all,
// rte_service_lcore_count,
// rte_service_lcore_count_services,
// rte_service_lcore_del,
// rte_service_lcore_list,
// rte_service_lcore_may_be_active,
// rte_service_lcore_reset_all,
// rte_service_lcore_start,
// rte_service_lcore_stop,
// rte_service_map_lcore_get,
// rte_service_map_lcore_set,
// rte_service_may_be_active,
// rte_service_probe_capability,
// rte_service_run_iter_on_app_lcore,
// rte_service_runstate_get,
// rte_service_runstate_set,
// rte_service_set_runstate_mapped_check,
// rte_service_set_stats_enable,
// rte_service_start_with_defaults,
// rte_set_application_usage_hook,
// rte_socket_count,
// rte_socket_id,
// rte_socket_id_by_idx,
// rte_srand,
// rte_strerror,
// rte_strscpy,
// rte_strsplit,
// rte_sys_gettid,
// rte_thread_get_affinity,
// rte_thread_is_intr,
// rte_thread_key_create,
// rte_thread_key_delete,
// rte_thread_register,
// rte_thread_set_affinity,
// rte_thread_setname,
// rte_thread_unregister,
// rte_thread_value_get,
// rte_thread_value_set,
// rte_uuid_compare,
// rte_uuid_is_null,
// rte_uuid_parse,
// rte_uuid_unparse,
// rte_vect_get_max_simd_bitwidth,
// rte_vect_set_max_simd_bitwidth,
// rte_version,
// rte_version_minor,
// rte_version_month,
// rte_version_prefix,
// rte_version_release,
// rte_version_suffix,
// rte_version_year,
// rte_vfio_container_dma_map,
// rte_vfio_container_dma_unmap,
// rte_vlog,
// rte_zmalloc,
// rte_zmalloc_socket,
