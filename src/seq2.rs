use core::marker::PhantomData;
use crate::store2::{ AsData, AccessSeq };


pub struct CompactSeq<SEQ, BUF>(PhantomData<(SEQ, BUF)>);

impl<SEQ, BUF> CompactSeq<SEQ, BUF> {
    pub const fn new() -> Self {
        CompactSeq(PhantomData)
    }
}

impl<
    const B: usize,
    SEQ,
    BUF,
> AccessSeq for CompactSeq<SEQ, BUF>
where
    SEQ: AccessSeq<Item = u32>,
    BUF: AsData<Data = [u8; B]>
{
    type Item = &'static [u8];

    const LEN: usize = SEQ::LEN;

    #[inline(always)]
    fn index(index: usize) -> Self::Item {
        let start: usize = index.checked_sub(1)
            .map(|index| SEQ::index(index))
            .unwrap_or_default()
            .try_into()
            .unwrap();
        let end: usize = SEQ::index(index)
            .try_into()
            .unwrap();
        &BUF::as_data()[start..end]
    }
}
