use std::{
    alloc::{Allocator, Global},
    cell::UnsafeCell,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use self::custom_hasher::IdentityHasher;

mod custom_hasher {
    use std::hash::{BuildHasher, Hasher};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct IdentityHasher(u64);

    impl Hasher for IdentityHasher {
        #[inline]
        fn finish(&self) -> u64 {
            self.0
        }

        #[inline]
        fn write_u64(&mut self, i: u64) {
            self.0 = i;
        }

        fn write(&mut self, _bytes: &[u8]) {
            unimplemented!("Only write u64s to this");
        }
    }

    impl BuildHasher for IdentityHasher {
        type Hasher = Self;

        fn build_hasher(&self) -> Self::Hasher {
            Self::default()
        }
    }
}

#[derive(Debug)]
pub struct PartitionedHashMap<
    K,
    V,
    const PARTITIONS: usize = 4,
    A = Global,
    HasherType = ahash::AHasher,
> where
    K: Hash + Eq,
    A: Allocator + Clone,
    HasherType: Hasher + Default,
    [V; PARTITIONS]: Sized, // non-zero
{
    tables: [UnsafeCell<hashbrown::HashMap<u64, V, IdentityHasher, A>>; PARTITIONS],
    refcount: [AtomicBool; PARTITIONS], // Used to make sure there is never more than 1 reference to a partition
    hasher: HasherType,
    _phantom: PhantomData<Arc<K>>,
}

impl<K, V, const PARTITIONS: usize, A, HasherType>
    PartitionedHashMap<K, V, PARTITIONS, A, HasherType>
where
    K: Hash + Eq,
    A: Allocator + Clone + Default,
    HasherType: Hasher + Default,
    [V; PARTITIONS]: Sized,
{
    pub fn new() -> Arc<Self> {
        Self::new_in(A::default())
    }

    pub fn with_capacity(capacity: usize) -> Arc<Self> {
        Self::with_capacity_in(capacity, A::default())
    }

    pub fn with_capacity_and_hasher(capacity: usize, hasher: HasherType) -> Arc<Self> {
        Self::with_capacity_and_hasher_in(capacity, hasher, A::default())
    }
}

const fn get_partitioning_mask(partitions: usize) -> u64 {
    if partitions.count_ones() != 1 {
        panic!("Table partition must be a power of 2");
    }
    let num_bits_to_partition = usize::BITS - partitions.leading_zeros() - 1;
    let bitmask_lower: u64 = (2 << (num_bits_to_partition - 1)) - 1;
    let leading_zeros: u64 = bitmask_lower.leading_zeros() as u64;
    bitmask_lower << leading_zeros
}

impl<K, V, const PARTITIONS: usize> PartitionedHashMap<K, V, PARTITIONS>
where
    K: Hash + Eq,
{
    #[inline]
    fn hash_key(&self, key: &K) -> u64 {
        let mut hasher = self.hasher.clone();
        key.hash(&mut hasher);
        hasher.finish()
    }

    // #[inline]
    pub fn get_partition_and_key_hash(&self, key: &K) -> (usize, u64) {
        let h = self.hash_key(key);
        let partitioning_mask = get_partitioning_mask(PARTITIONS);
        let mask_trailing_zeros = partitioning_mask.trailing_zeros();
        let partition = (partitioning_mask & h) >> mask_trailing_zeros;
        debug_assert!(partition <= PARTITIONS as u64);
        (partition as usize, h)
    }
}

impl<K, V, const PARTITIONS: usize, A, HasherType>
    PartitionedHashMap<K, V, PARTITIONS, A, HasherType>
where
    K: Hash + Eq,
    A: Allocator + Clone,
    HasherType: Hasher + Default,
{
    pub fn new_in(allocator: A) -> Arc<Self> {
        Self::with_capacity_in(0, allocator)
    }

    pub fn with_capacity_in(capacity: usize, allocator: A) -> Arc<Self> {
        Self::with_capacity_and_hasher_in(capacity, Default::default(), allocator)
    }

    pub fn with_capacity_and_hasher_in(
        capacity: usize,
        hasher: HasherType,
        allocator: A,
    ) -> Arc<Self> {
        Arc::new(Self {
            tables: std::array::from_fn(|_| {
                UnsafeCell::new(
                    hashbrown::HashMap::<u64, V, IdentityHasher, A>::with_capacity_and_hasher_in(
                        capacity,
                        IdentityHasher::default(),
                        allocator.clone(),
                    ),
                )
            }),
            hasher,
            _phantom: PhantomData::default(),
            refcount: std::array::from_fn(|_| AtomicBool::new(false)),
        })
    }

    fn make_handle(map: Arc<Self>, id: u64) -> PartitionedHashMapHandle<K, V, PARTITIONS, A, HasherType> {
        assert!(
            (id as usize) < PARTITIONS,
            "{id} is not a valid partition id, it is too large"
        );
        let flag = map.refcount[id as usize].compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        assert_eq!(Ok(false), flag, "Handle for {id} was already taken.");
        let table = map.tables[id as usize].get();
        PartitionedHashMapHandle::new(id, table, map)
    }

    pub fn create_all_handles(
        map: &Arc<Self>,
    ) -> [Option<PartitionedHashMapHandle<K, V, PARTITIONS, A, HasherType>>; PARTITIONS] {
        std::array::from_fn(|id| Some(Self::make_handle(map.clone(), id as u64)))
    }
}

#[derive(Debug)]
pub struct PartitionedHashMapHandle<K, V, const PARTITIONS: usize, A = Global, S = ahash::AHasher>
where
    K: Eq + Hash,
    A: Allocator + Clone,
    S: Hasher + Default,
{
    partition_id: u64,
    inner: *mut hashbrown::HashMap<u64, V, IdentityHasher, A>,
    _parent: Arc<PartitionedHashMap<K, V, PARTITIONS, A, S>>,
}

impl<K, V, const PARTITIONS: usize, A, S> PartitionedHashMapHandle<K, V, PARTITIONS, A, S>
where
    K: Eq + Hash,
    A: Allocator + Clone,
    S: Hasher + Default,
{
    pub fn new(
        partition_id: u64,
        inner: *mut hashbrown::HashMap<u64, V, IdentityHasher, A>,
        _parent: Arc<PartitionedHashMap<K, V, PARTITIONS, A, S>>,
    ) -> Self {
        Self {
            partition_id,
            inner,
            _parent,
        }
    }

    #[inline]
    pub fn check_key(&self, key: u64) {
        #[cfg(debug_assertions)]
        {
            let partitioning_mask = get_partitioning_mask(PARTITIONS);
            let expected_partition_id =
                (partitioning_mask & key) >> partitioning_mask.trailing_zeros();
            debug_assert_eq!(
                self.partition_id, expected_partition_id,
                "Invalid key {} passed to partition {}",
                key, self.partition_id
            );
        }
    }

    /// Returns a mutable reference to the get inner of this [`PartitionedHashMapHandle<V, PARTITIONS, PARTITION_ID, A>`].
    ///
    /// # Panics
    ///
    /// Panics in debug mode if a null pointer is supplied. This should never happen if everything works correctly,
    /// so if debug_assertions is off then it's an unchecked unwrap.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    fn get_inner(&self) -> &mut hashbrown::HashMap<u64, V, IdentityHasher, A> {
        let reference = unsafe { self.inner.as_mut() };
        #[cfg(debug_assertions)]
        {
            reference.unwrap()
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            reference.unwrap_unchecked()
        }
    }

    #[inline]
    pub fn get(&self, key: u64) -> Option<&V> {
        #[cfg(debug_assertions)]
        {
            self.check_key(key);
        }
        self.get_inner().get(&key)
    }

    #[inline]
    pub fn put(&self, key: u64, value: V) -> Option<V> {
        #[cfg(debug_assertions)]
        {
            self.check_key(key);
        }
        self.get_inner().insert(key, value)
    }

    #[inline]
    pub fn delete(&self, key: u64) -> Option<V> {
        #[cfg(debug_assertions)]
        {
            self.check_key(key);
        }
        self.get_inner().remove(&key)
    }

    #[inline]
    pub fn clear(&self) {
        self.get_inner().clear()
    }

    pub fn len(&self) -> usize {
        self.get_inner().len()
    }

    pub fn is_empty(&self) -> bool {
        self.get_inner().is_empty()
    }
}

unsafe impl<K, V, const PARTITIONS: usize, A> Send for PartitionedHashMapHandle<K, V, PARTITIONS, A>
where
    K: Eq + Hash,
    A: Allocator + Clone,
{
}

unsafe impl<K, V, const PARTITIONS: usize, A, HasherType> Sync
    for PartitionedHashMap<K, V, PARTITIONS, A, HasherType>
where
    K: Hash + Eq,
    A: Allocator + Clone,
    HasherType: Hasher + Default,
    [V; PARTITIONS]: Sized,
{
}
