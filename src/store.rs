use crate::MapStore;
use crate::aligned::AlignedArray;

pub struct List<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct RefList<'data, const N: usize, T> {
    data: &'data [T; N]
}

pub struct Compact<
    'data,
    const DATA: usize,
    SEQ
> {
    seq: SEQ,
    data: &'data [u8; DATA],
}

pub struct CompactStr<
    'data,
    SEQ
> {
    seq: SEQ,
    data: &'data str,
}

pub trait AccessList<'data> {
    type Item: 'data;
    const LEN: usize;

    fn index(&self, index: usize) -> Self::Item;
}

impl<'data, K, V> MapStore<'data> for (K, V)
where
    K: AccessList<'data>,
    V: AccessList<'data>
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

impl<'data, const N: usize, T: Copy> AccessList<'data> for List<'data, N, T> {
    type Item = T;

    const LEN: usize = N;

    fn index(&self, index: usize) -> Self::Item {
        self.data[index]
    }
}

impl<'data, const N: usize, T> AccessList<'data> for RefList<'data, N, T> {
    type Item = &'data T;

    const LEN: usize = N;

    fn index(&self, index: usize) -> Self::Item {
        &self.data[index]
    }
}

impl<
    'data,
    const DATA: usize,
    SEQ: AccessList<'data, Item = u32>,
> AccessList<'data> for Compact<'data, DATA, SEQ> {
    type Item = &'data [u8];

    const LEN: usize = SEQ::LEN;

    fn index(&self, index: usize) -> Self::Item {
        let start: usize = index.checked_sub(1)
            .map(|index| self.seq.index(index))
            .unwrap_or_default()
            .try_into()
            .unwrap();
        let end: usize = self.seq.index(index)
            .try_into()
            .unwrap();
        &self.data[start..end]
    }
}

impl<
    'data,
    SEQ: AccessList<'data, Item = u32>,
> AccessList<'data> for CompactStr<'data, SEQ> {
    type Item = &'data str;

    const LEN: usize = SEQ::LEN;

    fn index(&self, index: usize) -> Self::Item {
        let start: usize = index.checked_sub(1)
            .map(|index| self.seq.index(index))
            .unwrap_or_default()
            .try_into()
            .unwrap();
        let end: usize = self.seq.index(index)
            .try_into()
            .unwrap();
        &self.data[start..end]
    }
}

impl<
    'data,
    const B: usize,
> AccessList<'data> for AlignedArray<'data, B, u16> {
    type Item = u16;

    const LEN: usize = <AlignedArray<'data, B, u16>>::LEN;

    fn index(&self, index: usize) -> Self::Item {
        self.get(index).unwrap()
    }
}

impl<
    'data,
    const B: usize,
> AccessList<'data> for AlignedArray<'data, B, u32> {
    type Item = u32;

    const LEN: usize = <AlignedArray<'data, B, u32>>::LEN;

    fn index(&self, index: usize) -> Self::Item {
        self.get(index).unwrap()
    }
}

impl<
    'data,
    const B: usize,
> AccessList<'data> for AlignedArray<'data, B, u64> {
    type Item = u64;

    const LEN: usize = <AlignedArray<'data, B, u64>>::LEN;

    fn index(&self, index: usize) -> Self::Item {
        self.get(index).unwrap()
    }
}
