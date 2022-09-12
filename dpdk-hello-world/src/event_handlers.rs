use anyhow::Context;
use dpdk::device::{
    eth::dev::EthdevPortId,
    event::{
        eth::{
            rx::rx_adapter::{RxAdapter, RxAdapterId, RxAdapterQueueConfig},
            tx::tx_adapter::{TxAdapter, TxAdapterId},
        },
        EventDeviceId,
    },
};

pub fn setup_rx_adapter_worker(
    adapter_id: RxAdapterId,
    event_device_id: EventDeviceId,
    port_id: EthdevPortId,
    queue_config: RxAdapterQueueConfig,
    rx_queue_id: i32,
) -> Result<(), anyhow::Error> {
    let mut adapter = RxAdapter::new(adapter_id, event_device_id, port_id)?;
    adapter.add_rx_queue(queue_config, rx_queue_id)?;
    adapter.start_forwarding_packets();
    Ok(())
}

pub fn setup_tx_adapter_worker(
    adapter_id: TxAdapterId,
    event_device_id: EventDeviceId,
    eth_port_id: EthdevPortId,
    ethdev_queue_id: i32,
) -> Result<(), anyhow::Error> {
    let mut adapter = TxAdapter::new(adapter_id, event_device_id)
        .context(format!("Setting up TX adapter {adapter_id}"))?;
    adapter.add_tx_queue(eth_port_id, ethdev_queue_id);
    adapter.start_sending_packets();
    Ok(())
}
