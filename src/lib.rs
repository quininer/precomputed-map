#![cfg_attr(not(feature = "builder"), no_std)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "builder")]
pub mod builder;
pub mod store;
pub mod seq;
pub mod aligned;
pub mod phf;

pub mod macros;
pub mod equivalent;
pub mod seq2;
pub mod store2;
pub mod aligned2;

use core::marker::PhantomData;
use phf::HashOne;
use equivalent::{ Equivalent, Comparable, Hashable };


/// Tiny map
///
/// 0..16
pub struct TinyMap<K: 'static, V: 'static> {
    data: &'static [(K, V)],
}

impl<K: 'static, V: 'static> TinyMap<K, V> {
    pub const fn new(data: &'static [(K, V)]) -> TinyMap<K, V> {
        TinyMap { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get<Q>(&self, key: &Q)
        -> Option<&V>
    where
        Q: Comparable<K> + ?Sized
    {
        self.data.binary_search_by(|k| key.compare(&k.0).reverse())
            .ok()
            .map(|idx| &self.data[idx].1)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&K, &V)> + '_ {
        self.data.iter().map(|(k, v)| (k, v))
    }
}

/// Small map
///
/// 0..12
pub struct SmallMap<D, H> {
    seed: u64,
    _phantom: PhantomData<(D, H)>
}

impl<D, H> SmallMap<D, H>
where
    D: store2::MapStore,
    H: HashOne,
{
    pub const fn new(seed: u64) -> Self {
        SmallMap {
            seed,
            _phantom: PhantomData
        }
    }

    pub const fn len(&self) -> usize {
        D::LEN
    }    

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    #[inline]
    fn inner_get<Q>(&self, key: &Q) -> usize
    where
        Q: Hashable<H> + ?Sized,
    {
        let size: u32 = D::LEN.try_into().unwrap();

        let hash = key.hash(self.seed);
        let index = fast_reduct32(high(hash) ^ low(hash), size);
        index.try_into().unwrap()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<D::Value>
    where
        Q: Equivalent<D::Key> + Hashable<H> + ?Sized,
    {
        if self.is_empty() {
            return None;
        }
        
        let index = self.inner_get(key);
        if key.equivalent(&D::get_key(index)?) {
            D::get_value(index)
        } else {
            None
        }
    }

    pub const fn iter(&self) -> store2::MapIter<'_, D> {
        store2::MapIter::new()
    }    
}

/// Medium map
///
/// 1024..10M
pub struct MediumMap<
    const SLOTS: usize,
    P,
    R,
    D,
    H,
> {
    seed: u64,
    _phantom: PhantomData<(
        [u8; SLOTS],
        P, R, D, H
    )>
}

impl<
    const SLOTS: usize,
    P,
    R,
    D,
    H,
> MediumMap<SLOTS, P, R, D, H>
where
    P: store2::AccessSeq<Item = u8>,
    R: store2::AccessSeq<Item = u32>,
    D: store2::MapStore,
    H: HashOne
{
    pub const fn new(seed: u64) -> Self {
        MediumMap {
            seed,
            _phantom: PhantomData
        }
    }

    const _ASSERT: () = if SLOTS != D::LEN + R::LEN {
        panic!();
    };

    pub const fn len(&self) -> usize {
        D::LEN
    }    

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]    
    fn inner_get<Q>(&self, key: &Q) -> usize
    where
        Q: Hashable<H> + ?Sized,
    {
        let pilots_len: u32 = P::LEN.try_into().unwrap();
        let slots_len: u32 = SLOTS.try_into().unwrap();

        let hash = key.hash(self.seed);
        let bucket: usize = fast_reduct32(low(hash), pilots_len).try_into().unwrap();
        let pilot = P::index(bucket).unwrap();
        let pilot_hash = phf::hash_pilot(self.seed, pilot);

        fast_reduct32(
            high(hash) ^ high(pilot_hash) ^ low(pilot_hash),
            slots_len
        ).try_into().unwrap()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<D::Value>
    where
        Q: Equivalent<D::Key> + Hashable<H> + ?Sized,
    {
        #[cold]
        #[inline(always)]
        fn remap_and_index<R, D, Q>(index: usize, key: &Q)
        -> Option<D::Value>
        where
            R: store2::AccessSeq<Item = u32>,
            D: store2::MapStore,
            Q: Equivalent<D::Key> + ?Sized,
        {
            let index: usize = R::index(index - D::LEN).unwrap().try_into().unwrap();
            if key.equivalent(&D::get_key(index)?) {
                D::get_value(index)
            } else {
                None
            }
        }
                
        if self.is_empty() {
            return None;
        }
        
        let index = self.inner_get(key);

        if index < D::LEN {
            if key.equivalent(&D::get_key(index)?) {
                D::get_value(index)
            } else {
                None
            }
        } else {
            remap_and_index::<R, D, Q>(index, key)
        }
    }

    pub const fn iter(&self) -> store2::MapIter<'_, D> {
        store2::MapIter::new()
    }
}

// https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
#[inline]
fn fast_reduct32(x: u32, limit: u32) -> u32 {
    (((x as u64) * (limit as u64)) >> 32) as u32
}

#[inline]
fn low(v: u64) -> u32 {
    v as u32
}

#[inline]
fn high(v: u64) -> u32 {
    (v >> 32) as u32
}
