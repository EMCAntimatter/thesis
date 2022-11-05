use std::collections::LinkedList;

pub struct Assert<const B: bool>;
pub trait True {}
impl True for Assert<true> {}

/// A linked-list of slabs for holding data
///
/// Attempting to create a list with a slab size of zero is a compile-time error
pub struct SlabList<T, const SLAB_SIZE: usize = 100>
where
    Assert<{ SLAB_SIZE > 0 }>: True,
    T: Ord,
{
    internals: LinkedList<[T; SLAB_SIZE]>,
}

impl<T, const SLAB_SIZE: usize> SlabList<T, SLAB_SIZE>
where
    Assert<{ SLAB_SIZE > 0 }>: True,
    T: Ord,
{
    pub fn new() -> Self {
        Self {
            internals: Default::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.internals.iter().flat_map(|a| a.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.internals.iter_mut().flat_map(|a| a.iter_mut())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let internals_len = self.internals.len(); // Cached value, O(1)
        match internals_len.checked_sub(index / SLAB_SIZE) {
            Some(backwards_index) => {
                let slab = self
                    .internals
                    .iter() // Start iterating the list
                    .rev() // Count from the back since most of the time what we want will be near the end
                    .nth(backwards_index); // Get the request
                match slab {
                    Some(slab) => slab.get(index % SLAB_SIZE),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn get_mut(&self, index: usize) -> Option<&T> {
        let internals_len = self.internals.len(); // Cached value, O(1)
        match internals_len.checked_sub(index / SLAB_SIZE) {
            Some(backwards_index) => {
                let slab = self
                    .internals
                    .iter() // Start iterating the list
                    .rev() // Count from the back since most of the time what we want will be near the end
                    .nth(backwards_index); // Get the request
                match slab {
                    Some(slab) => slab.get(index % SLAB_SIZE),
                    None => None,
                }
            }
            None => None,
        }
    }

    // pub fn take_before(self, value: T) {
    // self.internals.into_iter().take_while(|slab| )
    // }
}

impl<T, const SLAB_SIZE: usize> Default for SlabList<T, SLAB_SIZE>
where
    Assert<{ SLAB_SIZE > 0 }>: True,
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}
