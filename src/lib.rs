#![cfg_attr(not(feature = "builder"), no_std)]

#[cfg(feature = "builder")]
pub mod builder;
mod store;
mod chd;
mod aligned;

use core::borrow::Borrow;
use core::hash::Hash;
use core::marker::PhantomData;
use store::AccessList;
pub use chd::{ HashOne, HashOne128 };

pub trait MapStore<'data> {
    type Key: 'data;
    type Value: 'data;

    const LEN: usize;
    
    fn get_key(&self, index: usize) -> Self::Key;
    fn get_value(&self, index: usize) -> Self::Value;
}

pub trait Searchable<'data>: MapStore<'data> {
    fn search<Q>(&self, key: &Q) -> Option<Self::Value>
    where
        Self::Key: Borrow<Q>,
        Q: Eq + ?Sized
    ;
}

/// Tiny map
///
/// 0..16
pub struct TinyMap<'data, D> {
    data: D,
    _phantom: PhantomData<&'data D>
}

impl<'data, D> TinyMap<'data, D>
where
    D: MapStore<'data> + Searchable<'data>,
    D::Key: Eq,
{
    pub const fn new(data: D) -> TinyMap<'data, D> {
        TinyMap { data, _phantom: PhantomData }
    }

    pub fn get<Q>(&self, key: &Q)
        -> Option<D::Value>
    where
        D::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        self.data.search(key)
    }
}

/// Small map
///
/// 16..1024
pub struct SmallMap<'data, D, H> {
    seed: u64,
    data: D,
    _phantom: PhantomData<&'data (D, H)>
}

impl<'data, D, H> SmallMap<'data, D, H>
where
    D: MapStore<'data>,
    D::Key: Hash + Eq + Copy,
    H: HashOne
{
    pub const fn new(seed: u64, data: D) -> SmallMap<'data, D, H> {
        SmallMap {
            seed, data,
            _phantom: PhantomData
        }
    }
    
    fn inner_get<Q>(&self, key: &Q) -> usize
    where
        Q: Hash + ?Sized
    {
        let size: u32 = D::LEN.try_into().unwrap();

        let hash = H::hash_one(self.seed, key) as u32;
        let index = fast_reduct32(hash, size);
        index.try_into().unwrap()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<D::Value>
    where
        D::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        let index = self.inner_get(key);
        if self.data.get_key(index).borrow() == key {
            Some(self.data.get_value(index))
        } else {
            None
        }
    }
}

/// Medium map
///
/// 1024..
pub struct MediumMap<'data, A, D, H> {
    seed: u64,
    disps: A,
    data: D,
    _phantom: PhantomData<&'data (A, D, H)>
}

impl<'data, A, D, H> MediumMap<'data, A, D, H>
where
    A: AccessList<'data, Item = u64>,
    D: MapStore<'data>,
    D::Key: Hash + Eq + Copy,
    H: HashOne128
{
    pub const fn new(seed: u64, disps: A, data: D)
        -> MediumMap<'data, A, D, H>
    {
        MediumMap {
            seed, disps, data,
            _phantom: PhantomData
        }
    }

    fn inner_get<Q>(&self, key: &Q) -> usize
    where
        Q: Hash + ?Sized
    {
        let len = D::LEN.try_into().unwrap();
        let disps_len: u32 = A::LEN.try_into().unwrap();
        
        let hash = H::hash_one128(self.seed, key);

        let (g, h1, h2) = chd::split_key(hash);
        let disps_idx = fast_reduct32(g, disps_len).try_into().unwrap();
        let disp = self.disps.index(disps_idx);
        let d0 = (disp >> 32) as u32;
        let d1 = disp as u32;

        let index = chd::displace(h1, h2, d0, d1);
        fast_reduct32(index, len).try_into().unwrap()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<D::Value>
    where
        D::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        let index = self.inner_get(key);
        if self.data.get_key(index).borrow() == key {
            Some(self.data.get_value(index))
        } else {
            None
        }
    }
}

// https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
fn fast_reduct32(x: u32, limit: u32) -> u32 {
    ((x as u64) * (limit as u64) >> 32) as u32
}
