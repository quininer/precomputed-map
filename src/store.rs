use core::marker::PhantomData;
use crate::DataStore;

pub struct Simple<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct Compact<'data, const N: usize> {
    bits: BitVec<'data>,
    data: &'data [u8],
    _phantom: PhantomData<[u8; N]>
}

pub struct BitVec<'data> {
    bits: &'data [u8]
}

pub struct Store<K, V>(pub K, pub V);

pub trait List {
    type Item: ?Sized;
    const LEN: usize;

    fn get(&self, index: usize) -> &Self::Item;
}

impl<'data, K, V> DataStore for Store<K, V>
where
    K: List,
    V: List
{
    type Key = K::Item;
    type Value = V::Item;

    const LEN: usize = {
        [(); 0][K::LEN - V::LEN];
        K::LEN
    };

    fn get_key(&self, index: usize) -> &Self::Key {
        self.0.get(index)
    }

    fn get_value(&self, index: usize) -> &Self::Value {
        self.1.get(index)
    }
}

impl<'data, const N: usize, T: 'data> List for Simple<'data, N, T> {
    type Item = T;

    const LEN: usize = N;

    fn get(&self, index: usize) -> &Self::Item {
        &self.data[index]
    }
}

impl<'data, const N: usize> List for Compact<'data, N> {
    type Item = [u8];

    const LEN: usize = N;

    fn get(&self, index: usize) -> &Self::Item {
        let (start, end) = self.bits.get_and_next(index).unwrap();
        &self.data[start..end]
    }
}

impl BitVec<'_> {
    pub fn get_and_next(&self, index: usize) -> Option<(usize, usize)> {
        todo!()
    }
}
