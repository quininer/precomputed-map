use core::mem;

pub struct AlignedBytes<const B: usize, T> {
    pub align: [T; 0],
    pub bytes: [u8; B]
}

#[derive(Clone, Copy)]
pub struct U64Array<'data, const B: usize> {
    pub bytes: &'data [u8; B]
    
}

impl<const B: usize> U64Array<'_, B> {
    pub const LEN: usize = {
        if B % mem::size_of::<u64>() != 0 {
            panic!();
        }

        B / mem::size_of::<u64>()
    };
    
    pub fn get_u64(&self, index: usize) -> Option<u64> {
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

// https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
pub fn fast_reduct64(x: u64, limit: u64) -> u64 {
    ((x as u128) * (limit as u128) >> 64) as u64
}

pub fn fast_reduct32(x: u32, limit: u32) -> u32 {
    ((x as u64) * (limit as u64) >> 32) as u32
}
