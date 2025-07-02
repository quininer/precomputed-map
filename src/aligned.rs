use core::mem;
use core::marker::PhantomData;
use crate::store::{ AsData, AccessSeq };

pub struct AlignedBytes<const B: usize, T> {
    pub align: [T; 0],
    pub bytes: [u8; B]
}

#[derive(Clone, Copy)]
pub struct AlignedArray<const B: usize, T, D> {
    _phantom: PhantomData<([T; B], D)>
}

impl<const B: usize, D> AlignedArray<B, u32, D>
where
    D: AsData<Data = [u8; B]>
{
    const ARRARY_LEN: usize = {
        if B % mem::size_of::<u32>() != 0 {
            panic!();
        }

        B / mem::size_of::<u32>()
    };
    
    #[inline(always)]
    pub fn get(index: usize) -> Option<u32> {
        let size = mem::size_of::<u32>();
        let index = index * size;

        debug_assert!(D::as_data().as_ptr().cast::<u32>().is_aligned());

        if B >= index + size {
            let buf = D::as_data()[index..][..size].try_into().unwrap();
            Some(u32::from_le_bytes(buf))
        } else {
            None
        }
    }
}

impl<const B: usize, D> AccessSeq for AlignedArray<B, u32, D>
where
    D: AsData<Data = [u8; B]>
{
    type Item = u32;
    const LEN: usize = Self::ARRARY_LEN;

    #[inline(always)]
    fn index(index: usize) -> Option<Self::Item> {
        Self::get(index)
    }
}
