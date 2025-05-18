use core::marker::PhantomData;
use crate::MapStore;

pub struct Simple<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct SimpleRef<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct NumberSeq<
    'data,
    const INDEX: usize,
    const LEN: usize,
> {
    data: &'data [u8; INDEX],
    _phantom: PhantomData<[(); LEN]>
}

pub struct Compact<
    'data,
    const DATA: usize,
    SEQ
> {
    seq: SEQ,
    data: &'data [u8; DATA],
}

pub trait List<'data> {
    type Item: 'data;
    const LEN: usize;

    fn get(&self, index: usize) -> Self::Item;
}

impl<'data, K, V> MapStore<'data> for (K, V)
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

impl<'data, const N: usize, T: Copy> List<'data> for Simple<'data, N, T> {
    type Item = T;

    const LEN: usize = N;

    fn get(&self, index: usize) -> Self::Item {
        self.data[index]
    }
}

impl<'data, const N: usize, T> List<'data> for SimpleRef<'data, N, T> {
    type Item = &'data T;

    const LEN: usize = N;

    fn get(&self, index: usize) -> Self::Item {
        &self.data[index]
    }
}

impl<
    'data,
    const INDEX: usize,
    const LEN: usize
> List<'data> for NumberSeq<'data, INDEX, LEN> {
    type Item = u32;

    const LEN: usize = LEN;

    fn get(&self, index: usize) -> Self::Item {
        todo!()
    }
}

impl<
    'data,
    const DATA: usize,
    SEQ: List<'data, Item = u32>,
> List<'data> for Compact<'data, DATA, SEQ> {
    type Item = &'data [u8];

    const LEN: usize = SEQ::LEN;

    fn get(&self, index: usize) -> Self::Item {
        let start: usize = index.checked_sub(1)
            .map(|index| self.seq.get(index))
            .unwrap_or_default()
            .try_into()
            .unwrap();
        let end: usize = self.seq.get(index)
            .try_into()
            .unwrap();
        &self.data[start..end]
    }
}
