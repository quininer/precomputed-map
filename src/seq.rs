use crate::aligned::AlignedArray;
use crate::store::ConstSlice;
use crate::AccessSeq;
use crate::store::AsData;

pub struct List<'data, const N: usize, T>(pub &'data [T; N]);
pub struct RefList<'data, const N: usize, T>(pub &'data [T; N]);

pub struct CompactSeq<
    'data,
    const O: usize,
    const L: usize,
    SEQ,
    BUF: ?Sized,
> {
    seq: SEQ,
    data: ConstSlice<'data, O, L, BUF>,
}

pub struct LimitedSeq<SEQ, BUF> {
    seq: SEQ,
    data: BUF,
}

impl<
    'data,
    const O: usize,
    const L: usize,
    SEQ,
    BUF: ?Sized,
> CompactSeq<'data, O, L, SEQ, BUF> {
    pub const fn new(seq: SEQ, data: ConstSlice<'data, O, L, BUF>) -> Self {
        CompactSeq { seq, data }
    }
}

impl<SEQ, BUF> LimitedSeq<SEQ, BUF> {
    pub const fn new(seq: SEQ, data: BUF) -> Self {
        LimitedSeq { seq, data }
    }
}

impl<'data, const N: usize, T: Copy> AccessSeq<'data> for List<'data, N, T> {
    type Item = T;

    const LEN: usize = N;

    #[inline]
    fn index(&self, index: usize) -> Self::Item {
        self.0[index]
    }
}

impl<'data, const N: usize, T> AccessSeq<'data> for RefList<'data, N, T> {
    type Item = &'data T;

    const LEN: usize = N;

    #[inline]
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
> AccessSeq<'data> for CompactSeq<'data, O, L, SEQ, [u8; B]>
where
    SEQ: AccessSeq<'data, Item = u32>,
{
    type Item = &'data [u8];

    const LEN: usize = SEQ::LEN;

    #[inline]
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
    const O: usize,
    const L: usize,
    SEQ,
> AccessSeq<'data> for CompactSeq<'data, O, L, SEQ, str>
where
    SEQ: AccessSeq<'data, Item = u32>,
{
    type Item = &'data str;

    const LEN: usize = SEQ::LEN;

    #[inline]
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

impl<'data, SEQ> AccessSeq<'data> for LimitedSeq<SEQ, &'data str>
where
    SEQ: AccessSeq<'data, Item = u32>,
{
    type Item = &'data str;

    const LEN: usize = SEQ::LEN;

    #[inline]
    fn index(&self, index: usize) -> Self::Item {
        let id = LimitedStr(self.seq.index(index));

        let offset = id.offset();
        let len = id.len();

        &self.data.as_data()[offset..][..len]        
    }
}

impl<
    'data,
    const B: usize,
    DATA
> AccessSeq<'data> for AlignedArray<B, u32, DATA>
where
    DATA: AsData<Data = &'data [u8; B]>
{
    type Item = u32;

    const LEN: usize = <AlignedArray<B, u32, DATA>>::LEN;

    #[inline]
    fn index(&self, index: usize) -> Self::Item {
        self.get(index).unwrap()
    }
}

/// Limited static string ID
/// 
/// 24bit offset and 8bit length
#[derive(Clone, Copy)]
pub struct LimitedStr(pub u32);

impl LimitedStr {
    pub fn offset(self) -> usize {
        (self.0 & ((1 << 24) - 1)).try_into().unwrap()
    }

    pub fn len(self) -> usize {
        (self.0 >> 24).try_into().unwrap()
    }    
}
