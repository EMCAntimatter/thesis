use crossbeam_utils::CachePadded;
use std::{
    alloc::{Allocator, Global},
    sync::atomic::AtomicUsize,
};

pub struct RustyRing<T, const SIZE: usize, A: Allocator = Global, AtomicCounterType = AtomicUsize>
where
    T: Sized + Unpin,
{
    producer: CachePadded<AtomicCounterType>,
    consumer: CachePadded<AtomicCounterType>,
    buffer: Box<[T; SIZE], A>,
}
