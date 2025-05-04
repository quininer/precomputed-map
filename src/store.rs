use core::marker::PhantomData;
use crate::MapStore;
use crate::util::U64Array;

pub struct Simple<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct Compact<'data, const B: usize, const N: usize> {
    data: &'data [u8],
    index: PhantomData<(&'data [u8; B], &'data [u8; N])>
}

pub struct Store<K, V>(pub K, pub V);

pub trait List<'data> {
    type Item: 'data;
    const LEN: usize;

    fn get(&self, index: usize) -> Self::Item;
}

impl<'data, K, V> MapStore<'data> for Store<K, V>
where
    K: List<'data>,
    V: List<'data>
{
    type Key = K::Item;
    type Value = V::Item;

    const LEN: usize = {
        [(); 0][K::LEN - V::LEN];
        K::LEN
    };

    fn get_key(&self, index: usize) -> Self::Key {
        self.0.get(index)
    }

    fn get_value(&self, index: usize) -> Self::Value {
        self.1.get(index)
    }
}

impl<'data, const N: usize, T> List<'data> for Simple<'data, N, T> {
    type Item = &'data T;

    const LEN: usize = N;

    fn get(&self, index: usize) -> Self::Item {
        &self.data[index]
    }
}

impl<'data, const B: usize, const N: usize> List<'data> for Compact<'data, B, N> {
    type Item = &'data [u8];

    const LEN: usize = N;

    fn get(&self, index: usize) -> Self::Item {
        todo!()
    }
}

impl<'data, const B: usize> List<'data> for U64Array<'data, B> {
    type Item = u64;

    const LEN: usize = <U64Array<'data, B>>::LEN;

    fn get(&self, index: usize) -> Self::Item {
        self.get_u64(index).unwrap()
    }
}
