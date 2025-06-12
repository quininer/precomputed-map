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
    #[inline]
    fn as_data(&self) -> &'data [u8; N] {
        self.data[O..][..N].try_into().unwrap()
    }
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const L: usize,
> AccessSeq<'data> for ConstSlice<'data, B, O, L> {
    type Item = u8;
    const LEN: usize = L;

    #[inline]
    fn index(&self, index: usize) -> Self::Item {
        self.as_data()[index]
    }
}

impl<'data, K> MapStore<'data> for K
where
    K: AccessSeq<'data>
{
    type Key = K::Item;
    type Value = usize;

    const LEN: usize = K::LEN;

    #[inline]
    fn get_key(&self, index: usize) -> Self::Key {
        self.index(index)
    }

    #[inline]
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
        if K::LEN != V::LEN {
            panic!();
        }

        K::LEN
    };

    #[inline]
    fn get_key(&self, index: usize) -> Self::Key {
        self.0.index(index)
    }

    #[inline]
    fn get_value(&self, index: usize) -> Self::Value {
        self.1.index(index)
    }
}
