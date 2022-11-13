use std::{
    collections::{BTreeMap, LinkedList},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    }, cmp,
};

use dpdk::memory::allocator::DPDKBox;

use core::fmt::Debug;

use itertools::Itertools;
use tracing::instrument;

use crate::message::client_message::{ClientId, ClientLogMessage, MessageId};

use super::pipeline::{SpscConsumerChannelHandle, SpscProducerChannelHandle};

pub trait Reorderable {
    type ReorderableKey: Into<usize>;

    fn key(&self) -> Self::ReorderableKey;
}

/// Parallel when partitioned by client ID
#[instrument(skip(input_channel, out_channel, next_holder))]
pub fn order_messages<LogKeyType, LogValueType>(
    mut input_channel: SpscConsumerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>,
    mut out_channel: SpscProducerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>,
    next_holder: Arc<AtomicU32>,
    client_id: ClientId,
) -> Result<(), anyhow::Error>
where
    LogKeyType: Debug,
    LogValueType: Debug,
{
    // let mut ordering_list = LinkedList::new();
    let mut ordering_list = BTreeMap::new();
    loop {
        if input_channel.is_abandoned() && input_channel.is_empty() && ordering_list.is_empty() {
            tracing::info!("Input channel abandoned and empty, closing");
            return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
        }
        while out_channel.slots() == 0 {
            if input_channel.is_abandoned() && input_channel.is_empty() {
                tracing::info!("Input channel abandoned and empty, closing");
                return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
            }
        }
        let message_batch = input_channel
            .read_chunk(input_channel.slots())
            .unwrap()
            .into_iter();
        // ordering_list = process_batch(message_batch, &next_holder, &mut out_channel, ordering_list);
        ordering_list = process_batch_with_tree_map(
            message_batch,
            &next_holder,
            &mut out_channel,
            ordering_list,
        );
    }
}

#[instrument(skip_all)]
fn process_batch<LogKeyType, LogValueType>(
    message_batch: rtrb::chunks::ReadChunkIntoIter<ClientLogMessage<LogKeyType, LogValueType>>,
    next_holder: &Arc<AtomicU32>,
    out_channel: &mut rtrb::Producer<ClientLogMessage<LogKeyType, LogValueType>>,
    mut ordering_list: LinkedList<ClientLogMessage<LogKeyType, LogValueType>>,
) -> LinkedList<ClientLogMessage<LogKeyType, LogValueType>>
where
    LogKeyType: Debug,
    LogValueType: Debug,
{
    for msg in message_batch {
        let _msg_span = tracing::trace_span!("msg_ordering", msg_id = msg.message_id.0).entered();
        tracing::trace!(event = "Got new message", msg = %msg);
        let mut next = next_holder.load(Ordering::Acquire);
        if msg.message_id.0 == next {
            tracing::trace!(event = "Message found to be the next");
            while out_channel.slots() == 0 {}
            ordering_list.push_front(msg);

            let mut cursor = ordering_list.cursor_front_mut();
            while let Some(msg) = cursor.current() && msg.message_id.0 == next {
                next += 1;
                next_holder.fetch_add(1, Ordering::AcqRel);
                cursor.move_next();
            }

            let mut to_insert = cursor.split_before();
            while !to_insert.is_empty() {
                let out_channel_slots = out_channel.slots();
                if out_channel_slots == 0 {
                    continue;
                }
                let num_to_write_this_round = out_channel_slots.min(to_insert.len());
                let output_chunk = out_channel
                    .write_chunk_uninit(num_to_write_this_round)
                    .unwrap();

                // This little mess happens because split_off returns everything AFTER
                // the provided index, instead of before. This is functionally
                // placing the first `num_to_write_this_round` elements into
                // `messages_to_write_this_round` and everything else is kept in
                // `to_insert`
                let new_to_insert = to_insert.split_off(num_to_write_this_round);
                let messages_to_write_this_round = to_insert;
                to_insert = new_to_insert;

                let num_written = output_chunk.fill_from_iter(messages_to_write_this_round);
                debug_assert_eq!(
                    num_to_write_this_round, num_written,
                    "Not all messages were written to output chunk"
                );
            }
        } else {
            // Unhappy path
            let mut cursor = ordering_list.cursor_back_mut();
            while let Some(cur) = cursor.current() && cur.message_id > msg.message_id {
                    cursor.move_prev()
                }
            cursor.insert_after(msg);
            tracing::trace!(
                event = "Message was not next, adding to buffer",
                buffer_len = ordering_list.len()
            );
        }
    }
    return ordering_list;
}

#[instrument(skip_all)]
fn process_batch_with_tree_map<LogKeyType, LogValueType>(
    message_batch: rtrb::chunks::ReadChunkIntoIter<ClientLogMessage<LogKeyType, LogValueType>>,
    next_holder: &Arc<AtomicU32>,
    out_channel: &mut rtrb::Producer<ClientLogMessage<LogKeyType, LogValueType>>,
    mut ordering_list: BTreeMap<MessageId, ClientLogMessage<LogKeyType, LogValueType>>,
) -> BTreeMap<MessageId, ClientLogMessage<LogKeyType, LogValueType>>
where
    LogKeyType: Debug,
    LogValueType: Debug,
{
    let sorted_iter = message_batch.sorted_unstable_by_key(|msg| msg.message_id);
    for msg in sorted_iter {
        ordering_list.insert(msg.message_id, msg);
    }

    let last_ordered = ordering_list
        .iter()
        .take_while(|msg| {
            let next = next_holder.load(Ordering::SeqCst);
            if msg.0 .0 == next {
                next_holder.fetch_add(1, Ordering::SeqCst);
                true
            } else {
                false
            }
        })
        .last()
        .map(|p| *p.0 + 1u32);

    if let Some(last_ordered_msg_id) = last_ordered {
        // This little mess happens because split_off returns everything AFTER
        // the provided index, instead of before. This is functionally
        // placing the first `num_to_write_this_round` elements into
        // `messages_to_write_this_round` and everything else is kept in
        // `to_insert`
        let new_ordering_list = ordering_list.split_off(&last_ordered_msg_id);
        let mut to_insert = ordering_list;
        ordering_list = new_ordering_list;

        while !to_insert.is_empty() {
            let current_message_id = to_insert.iter().take(1).last().unwrap().0;

            let out_channel_slots = out_channel.slots();
            if out_channel_slots == 0 {
                continue;
            }
            let num_to_write_this_round = out_channel_slots.min(to_insert.len());
            let output_chunk = out_channel
                .write_chunk_uninit(num_to_write_this_round)
                .unwrap();

            // This little mess happens because split_off returns everything AFTER
            // the provided index, instead of before. This is functionally
            // placing the first `num_to_write_this_round` elements into
            // `messages_to_write_this_round` and everything else is kept in
            // `to_insert`
            let message_id_to_split_on =
                MessageId(current_message_id.0 + num_to_write_this_round as u32);
            let new_to_insert = to_insert.split_off(&message_id_to_split_on);
            let messages_to_write_this_round = to_insert;
            to_insert = new_to_insert;

            let iter = messages_to_write_this_round.into_values();
            let num_written = output_chunk.fill_from_iter(iter);
            debug_assert_eq!(
                num_to_write_this_round, num_written,
                "Not all messages were written to output chunk"
            );
        }
    }
    return ordering_list;
}
