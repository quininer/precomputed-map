use core::mem;
use core::marker::PhantomData;
use crate::store::AsData;

pub struct AlignedBytes<const B: usize, T> {
    pub align: [T; 0],
    pub bytes: [u8; B]
}

#[derive(Clone, Copy)]
pub struct AlignedArray<const B: usize, T, DATA> {
    pub bytes: DATA,
    _phantom: PhantomData<[T; B]>
}

impl<'data, const B: usize, DATA> AlignedArray<B, u32, DATA>
where
    DATA: AsData<'data, B>
{
    pub const LEN: usize = {
        if B % mem::size_of::<u32>() != 0 {
            panic!();
        }

        B / mem::size_of::<u32>()
    };

    pub const fn new(bytes: DATA) -> Self {
        AlignedArray { bytes, _phantom: PhantomData }
    }
    
    #[inline]
    pub fn get(&self, index: usize) -> Option<u32> {
        let size = mem::size_of::<u32>();
        let index = index * size;

        if B >= index + size {
            let buf = self.bytes.as_data()[index..][..size].try_into().unwrap();
            Some(u32::from_le_bytes(buf))
        } else {
            None
        }
    }
}
