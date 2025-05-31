use core::mem;
use core::marker::PhantomData;

pub struct AlignedBytes<const B: usize, T> {
    pub align: [T; 0],
    pub bytes: [u8; B]
}

#[derive(Clone, Copy)]
pub struct AlignedArray<'data, const B: usize, T> {
    pub bytes: &'data [u8; B],
    _phantom: PhantomData<T>
}

impl<const B: usize> AlignedArray<'_, B, u32> {
    pub const LEN: usize = {
        if B % mem::size_of::<u32>() != 0 {
            panic!();
        }

        B / mem::size_of::<u32>()
    };
    
    pub fn get(&self, index: usize) -> Option<u32> {
        let size = mem::size_of::<u32>();
        let index = index * size;

        if B >= index + size {
            let buf = self.bytes[index..][..size].try_into().unwrap();
            Some(u32::from_le_bytes(buf))
        } else {
            None
        }
    }
}

impl<const B: usize> AlignedArray<'_, B, u64> {
    pub const LEN: usize = {
        if B % mem::size_of::<u64>() != 0 {
            panic!();
        }

        B / mem::size_of::<u64>()
    };
    
    pub fn get(&self, index: usize) -> Option<u64> {
        let size = mem::size_of::<u64>();
        let index = index * size;

        if B >= index + size {
            let buf = self.bytes[index..][..size].try_into().unwrap();
            Some(u64::from_le_bytes(buf))
        } else {
            None
        }
    }
}
