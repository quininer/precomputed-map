use crate::aligned::AlignedArray;
use crate::store::ConstSlice;
use crate::AccessSeq;
use crate::AsData;

pub struct List<'data, const N: usize, T>(pub &'data [T; N]);
pub struct RefList<'data, const N: usize, T>(pub &'data [T; N]);

pub struct CompactSeq<
    'data,
    const B: usize,
    const O: usize,
    const L: usize,
    SEQ,
> {
    seq: SEQ,
    data: ConstSlice<'data, B, O, L>,
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const L: usize,
    SEQ,
> CompactSeq<'data, B, O, L, SEQ> {
    pub const fn new(seq: SEQ, data: ConstSlice<'data, B, O, L>) -> Self {
        CompactSeq { seq, data }
    }
}

impl<'data, const N: usize, T: Copy> AccessSeq<'data> for List<'data, N, T> {
    type Item = T;

    const LEN: usize = N;

    fn index(&self, index: usize) -> Self::Item {
        self.0[index]
    }
}

impl<'data, const N: usize, T> AccessSeq<'data> for RefList<'data, N, T> {
    type Item = &'data T;

    const LEN: usize = N;

    fn index(&self, index: usize) -> Self::Item {
        &self.0[index]
    }
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const L: usize,
    SEQ,
> AccessSeq<'data> for CompactSeq<'data, B, O, L, SEQ>
where
    SEQ: AccessSeq<'data, Item = u32>,
{
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
        &self.data.as_data()[start..end]
    }
}

impl<
    'data,
    const B: usize,
    DATA
> AccessSeq<'data> for AlignedArray<B, u32, DATA>
where
    DATA: AsData<'data, B>
{
    type Item = u32;

    const LEN: usize = <AlignedArray<B, u32, DATA>>::LEN;

    fn index(&self, index: usize) -> Self::Item {
        self.get(index).unwrap()
    }
}
