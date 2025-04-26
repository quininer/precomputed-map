#![cfg_attr(not(feature = "builder"), no_std)]

#[cfg(feature = "builder")]
pub mod builder;
mod store;

mod util;

use core::hash::{ Hasher, Hash };
use core::marker::PhantomData;

pub trait DataStore {
    type Key: ?Sized;
    type Value: ?Sized;

    const LEN: usize;
    
    fn get_key(&self, index: usize) -> &Self::Key;
    fn get_value(&self, index: usize) -> &Self::Value;
}

pub trait Searchable: DataStore {
    fn search(&self, key: &Self::Key) -> Option<usize>;
}

/// Tiny map
///
/// 0..16
pub struct TinyMap<D>(D);

impl<D> TinyMap<D>
where
    D: DataStore + Searchable,
    D::Key: Eq,
{
    pub const fn new(data: D) -> TinyMap<D> {
        TinyMap(data)
    }

    pub fn get<'a>(&'a self, key: &D::Key) -> Option<&'a D::Value> {
        self.0.search(key).map(|index| self.0.get_value(index))
    }
}

/// Small map
///
/// 16..1024
pub struct SmallMap<D, H> {
    seed: u64,
    data: D,
    _phantom: PhantomData<H>
}

impl<D, H> SmallMap<D, H>
where
    D: DataStore,
    D::Key: Hash + Eq,
    H: Hasher + Default,
{
    pub const fn new(seed: u64, data: D) -> SmallMap<D, H> {
        SmallMap {
            seed, data,
            _phantom: PhantomData
        }
    }
    
    fn inner_get(&self, key: &D::Key) -> usize {
        let size: u64 = D::LEN.try_into().unwrap();

        let mut hasher = H::default();
        self.seed.hash(&mut hasher);
        key.hash(&mut hasher);
        let index = hasher.finish() % size;
        index.try_into().unwrap()
    }

    pub fn get<'a>(&'a self, key: &D::Key) -> Option<&'a D::Value> {
        let index = self.inner_get(key);
        if self.data.get_key(index) == key {
            Some(self.data.get_value(index))
        } else {
            None
        }
    }
}

/// Medium map
///
/// 1024..65536
pub struct MediumMap<D, H> {
    seed: u64,
    data: D,
    _phantom: PhantomData<H>
}

impl<D, H> MediumMap<D, H>
where
    D: DataStore,
    D::Key: Hash + Eq,
    H: Hasher + Default,
{
    pub const fn new(seed: u64, data: D) -> MediumMap<D, H> {
        MediumMap {
            seed, data,
            _phantom: PhantomData
        }
    }

    pub fn get<'a>(&'a self, key: &D::Key) -> Option<&'a D::Value> {
        todo!()
    }
}

