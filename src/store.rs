use core::marker::PhantomData;
use crate::{ MapStore, AccessSeq, AsData };

pub struct ConstSlice<'data, const B: usize, const O: usize, const L: usize> {
    data: &'data [u8; B],
    _phantom: PhantomData<([u8; O], [u8; L])>
}

impl<'data, const B: usize, const O: usize, const L: usize> ConstSlice<'data, B, O, L> {
    pub const fn new(data: &'data [u8; B]) -> Self {
        ConstSlice { data, _phantom: PhantomData }
    }
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const N: usize,
> AsData<'data, N> for ConstSlice<'data, B, O, N> {
    fn as_data(&self) -> &'data [u8; N] {
        self.data[O..][..N].try_into().unwrap()
    }
}

pub struct Indexed<K>(pub K);

impl<'data, K> MapStore<'data> for Indexed<K>
where
    K: AccessSeq<'data>
{
    type Key = K::Item;
    type Value = usize;

    const LEN: usize = K::LEN;

    fn get_key(&self, index: usize) -> Self::Key {
        self.0.index(index)
    }

    fn get_value(&self, index: usize) -> Self::Value {
        index
    }
}

impl<'data, K, V> MapStore<'data> for (K, V)
where
    K: AccessSeq<'data>,
    V: AccessSeq<'data>
{
    type Key = K::Item;
    type Value = V::Item;

    const LEN: usize = {
        [(); 0][K::LEN - V::LEN];
        K::LEN
    };

    fn get_key(&self, index: usize) -> Self::Key {
        self.0.index(index)
    }

    fn get_value(&self, index: usize) -> Self::Value {
        self.1.index(index)
    }
}
