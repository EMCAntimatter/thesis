use std::{
    alloc::{Allocator, Global},
    collections::BTreeMap,
    hash::Hash,
};

pub trait State<K, V, A: Allocator = Global> {
    fn get(&mut self, key: &K) -> Option<&V>;
    fn put(&mut self, key: K, value: V) -> Option<V>;
    fn delete(&mut self, key: &K) -> Option<V>;
    fn clear(&mut self);
}

impl<K, V, Hasher, A> State<K, V, A> for hashbrown::HashMap<K, V, Hasher, A>
where
    K: Eq + Hash,
    Hasher: std::hash::BuildHasher,
    A: Allocator + Clone,
{
    fn get(&mut self, key: &K) -> Option<&V> {
        hashbrown::HashMap::get(self, key)
    }

    fn put(&mut self, key: K, value: V) -> Option<V> {
        hashbrown::HashMap::insert(self, key, value)
    }

    fn delete(&mut self, key: &K) -> Option<V> {
        hashbrown::HashMap::remove(self, key)
    }

    fn clear(&mut self) {
        hashbrown::HashMap::clear(self)
    }
}

impl<K, V, A> State<K, V, A> for BTreeMap<K, V, A>
where
    K: Ord,
    A: Allocator + Clone,
{
    fn get(&mut self, key: &K) -> Option<&V> {
        BTreeMap::get(self, key)
    }

    fn put(&mut self, key: K, value: V) -> Option<V> {
        BTreeMap::insert(self, key, value)
    }

    fn delete(&mut self, key: &K) -> Option<V> {
        BTreeMap::remove(self, key)
    }

    fn clear(&mut self) {
        BTreeMap::clear(self)
    }
}

impl<K, V, Hasher, A> State<K, V, A> for std::collections::HashMap<K, V, Hasher>
where
    K: Eq + Hash,
    Hasher: std::hash::BuildHasher,
    A: Allocator + Clone,
{
    fn get(&mut self, key: &K) -> Option<&V> {
        std::collections::HashMap::get(self, key)
    }

    fn put(&mut self, key: K, value: V) -> Option<V> {
        std::collections::HashMap::insert(self, key, value)
    }

    fn delete(&mut self, key: &K) -> Option<V> {
        std::collections::HashMap::remove(self, key)
    }

    fn clear(&mut self) {
        std::collections::HashMap::clear(self)
    }
}
