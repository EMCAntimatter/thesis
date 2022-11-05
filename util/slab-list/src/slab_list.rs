use std::{
    alloc::{Allocator, Global},
    collections::LinkedList,
};

type Slab<T, const SLAB_SIZE: usize, A> = Box<[Option<T>; SLAB_SIZE], A>;
type SlabListBacking<T, const SLAB_SIZE: usize, A> = LinkedList<Slab<T, SLAB_SIZE, A>>;

#[derive(Debug, Default)]
pub struct SlabList<T, const SLAB_SIZE: usize = 1600, A = Global>
where
    T: Sized,
    A: Allocator + Default,
{
    num_slabs_freed: usize,
    num_elements_freed: usize,
    inner: SlabListBacking<T, SLAB_SIZE, A>,
}

impl<T, const SLAB_SIZE: usize, A> SlabList<T, SLAB_SIZE, A>
where
    T: Sized,
    A: Allocator + Default,
{
    pub fn new() -> Self {
        let first_slab = Box::new_zeroed_in(A::default());
        // Zeroing everything makes Option None.
        let first_slab = unsafe { first_slab.assume_init() };
        let inner = LinkedList::from([first_slab]);
        Self {
            num_slabs_freed: 0,
            num_elements_freed: 0,
            inner,
        }
    }

    #[inline]
    fn num_elements_in_current_slab_freed(&mut self) -> usize {
        self.num_elements_freed * SLAB_SIZE
    }

    fn global_index_to_slab_index(&self, index: usize) -> (usize, usize) {
        let slab = index.div_floor(SLAB_SIZE) - self.num_slabs_freed;
        debug_assert!(index.div_floor(SLAB_SIZE) >= self.num_slabs_freed);
        let index = index - self.num_elements_freed;
        debug_assert!(index < SLAB_SIZE);
        (slab, index)
    }

    fn add_n_slabs(&mut self, n: usize) {
        for _ in 0..n {
            let slab = Box::new_zeroed_in(A::default());
            let slab = unsafe { slab.assume_init() };
            self.inner.push_back(slab);
        }
    }

    #[inline]
    pub fn insert_at(&mut self, index: usize, val: T) {
        let (slab, index_in_slab) = self.global_index_to_slab_index(index);
        debug_assert!(index_in_slab >= self.num_elements_in_current_slab_freed());
        let current_num_slabs = self.inner.len();
        if current_num_slabs < slab {
            self.add_n_slabs(slab - current_num_slabs);
        }
        // Safe to unwrap since I just made sure there are enough slabs.
        let slab_to_insert_into = self.inner.iter_mut().nth(slab).unwrap();
        slab_to_insert_into[index_in_slab].replace(val);
    }
}
