#![cfg_attr(not(feature = "builder"), no_std)]

#[cfg(feature = "builder")]
pub mod builder;
mod store;
mod phf;
mod aligned;

use core::borrow::Borrow;
use core::hash::Hash;
use core::marker::PhantomData;
use store::AccessList;
pub use phf::{ HashOne, U64Hasher };

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

        let hash = H::hash_one(self.seed, key);
        let index = fast_reduct32(high(hash) ^ high(hash), size);
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
/// 1024..10M
pub struct MediumMap<'data, P, R, D, H> {
    seed: u64,
    pilots: P,
    remap: R,
    data: D,
    _phantom: PhantomData<&'data (P, D, R, H)>
}

impl<'data, P, R, D, H> MediumMap<'data, P, R, D, H>
where
    P: AccessList<'data, Item = u8>,
    R: AccessList<'data, Item = u32>,
    D: MapStore<'data>,
    D::Key: Hash + Eq + Copy,
    H: HashOne
{
    pub const fn new(seed: u64, pilots: P, remap: R, data: D)
        -> MediumMap<'data, P, R, D, H>
    {
        MediumMap {
            seed, pilots, remap, data,
            _phantom: PhantomData
        }
    }

    fn inner_get<Q>(&self, key: &Q) -> usize
    where
        Q: Hash + ?Sized
    {
        let pilots_len: u32 = P::LEN.try_into().unwrap();
        let slots_len: u32 = 0;
        
        let hash = H::hash_one(self.seed, key);
        let bucket: usize = fast_reduct32(low(hash), pilots_len).try_into().unwrap();
        let pilot = self.pilots.index(bucket);
        let pilot_hash = phf::hash_pilot(self.seed, pilot);
        let index: usize = fast_reduct32(high(hash) ^ high(pilot_hash) ^ low(pilot_hash), slots_len).try_into().unwrap();

        match index.checked_sub(D::LEN) {
            None => index,
            Some(offset) => self.remap.index(offset).try_into().unwrap()
        }
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

fn low(v: u64) -> u32 {
    v as u32
}

fn high(v: u64) -> u32 {
    (v >> 32) as u32
}
