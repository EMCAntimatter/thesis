use dpdk_sys::{
    rte_event, rte_event__bindgen_ty_1, rte_event__bindgen_ty_1__bindgen_ty_1,
    rte_event__bindgen_ty_2, rte_event_vector, rte_mbuf, RTE_EVENT_OP_FORWARD, RTE_EVENT_OP_NEW,
    RTE_EVENT_OP_RELEASE, RTE_EVENT_TYPE_CPU, RTE_EVENT_TYPE_CPU_VECTOR, RTE_EVENT_TYPE_CRYPTODEV,
    RTE_EVENT_TYPE_ETHDEV, RTE_EVENT_TYPE_ETHDEV_VECTOR, RTE_EVENT_TYPE_ETH_RX_ADAPTER,
    RTE_EVENT_TYPE_ETH_RX_ADAPTER_VECTOR, RTE_EVENT_TYPE_TIMER, RTE_SCHED_TYPE_ATOMIC,
    RTE_SCHED_TYPE_ORDERED, RTE_SCHED_TYPE_PARALLEL,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

#[derive(Debug, Default, Clone, Copy, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum EventType {
    #[default]
    EthDev = RTE_EVENT_TYPE_ETHDEV as u8,
    CryptoDev = RTE_EVENT_TYPE_CRYPTODEV as u8,
    CPU = RTE_EVENT_TYPE_CPU as u8,
    EthRxAdapter = RTE_EVENT_TYPE_ETH_RX_ADAPTER as u8,
    Timer = RTE_EVENT_TYPE_TIMER as u8,
    EthDevVector = RTE_EVENT_TYPE_ETHDEV_VECTOR as u8,
    CpuVector = RTE_EVENT_TYPE_CPU_VECTOR as u8,
    EthRxAdapterVector = RTE_EVENT_TYPE_ETH_RX_ADAPTER_VECTOR as u8,
}

#[derive(Debug, Default, Clone, Copy, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum EventOp {
    #[default]
    New = RTE_EVENT_OP_NEW as u8,
    Forward = RTE_EVENT_OP_FORWARD as u8,
    Release = RTE_EVENT_OP_RELEASE as u8,
}

#[derive(Debug, Default, Clone, Copy, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum EventScheduleType {
    #[default]
    Ordered = RTE_SCHED_TYPE_ORDERED as u8,
    Atomic = RTE_SCHED_TYPE_ATOMIC as u8,
    Parallel = RTE_SCHED_TYPE_PARALLEL as u8,
}

#[derive(Debug, Clone, Copy)]
pub enum EventWordZero {
    Event(u64),
    EventAttributes {
        queue_id: u8,
        flow_id: u32,
        event_type: EventType,
        event_subtype: u8,
        operation: EventOp,
        scheduling_type: EventScheduleType,
        priority: u8,
    },
}

impl Default for EventWordZero {
    fn default() -> Self {
        Self::Event(0)
    }
}

#[derive(Debug)]
pub enum EventWordOne {
    Opaque64BitValue(u64),
    EventPtr(*mut libc::c_void),
    MBufPtr(*mut rte_mbuf),
    EventVectorPtr(*mut rte_event_vector),
}

impl Default for EventWordOne {
    fn default() -> Self {
        Self::Opaque64BitValue(0)
    }
}

#[derive(Debug, Default)]
pub struct Event {
    pub word0: EventWordZero,
    pub word1: EventWordOne,
}

impl From<&Event> for rte_event {
    fn from(val: &Event) -> Self {
        let word0 = match val.word0 {
            EventWordZero::Event(event) => rte_event__bindgen_ty_1 { event },
            EventWordZero::EventAttributes {
                queue_id,
                flow_id,
                event_type,
                event_subtype,
                operation,
                scheduling_type,
                priority,
            } => {
                let mut attributes = rte_event__bindgen_ty_1__bindgen_ty_1 {
                    _bitfield_align_1: [],
                    _bitfield_1: Default::default(),
                    queue_id,
                    priority,
                    impl_opaque: 0,
                };

                attributes.set_flow_id(flow_id);
                attributes.set_sub_event_type(event_subtype as u32);
                attributes.set_event_type(event_type.to_u32().unwrap());
                attributes.set_op(operation.to_u8().unwrap());
                attributes.set_sched_type(scheduling_type.to_u8().unwrap());

                rte_event__bindgen_ty_1 {
                    __bindgen_anon_1: attributes,
                }
            }
        };

        let word1 = match val.word1 {
            EventWordOne::Opaque64BitValue(value) => rte_event__bindgen_ty_2 { u64_: value },
            EventWordOne::EventPtr(event_ptr) => rte_event__bindgen_ty_2 { event_ptr },
            EventWordOne::MBufPtr(mbuf) => rte_event__bindgen_ty_2 { mbuf },
            EventWordOne::EventVectorPtr(vec) => rte_event__bindgen_ty_2 { vec },
        };

        rte_event {
            __bindgen_anon_1: word0,
            __bindgen_anon_2: word1,
        }
    }
}
