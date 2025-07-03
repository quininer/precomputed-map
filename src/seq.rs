use core::marker::PhantomData;
use crate::store::{ AsData, AccessSeq };


pub struct PositionSeq<SEQ, BUF>(PhantomData<(SEQ, BUF)>);

impl<
    const B: usize,
    SEQ,
    BUF,
> AccessSeq for PositionSeq<SEQ, BUF>
where
    SEQ: AccessSeq<Item = u32>,
    BUF: AsData<Data = [u8; B]>
{
    type Item = &'static [u8];

    const LEN: usize = SEQ::LEN;

    #[inline(always)]
    fn index(index: usize) -> Option<Self::Item> {
        let start: usize = match index.checked_sub(1) {
            Some(index) => SEQ::index(index)?.try_into().unwrap(),
            None => 0
        };
        let end: usize = SEQ::index(index)?
            .try_into()
            .unwrap();
        BUF::as_data().get(start..end)
    }
}
