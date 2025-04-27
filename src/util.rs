use core::mem;

pub struct AlignedBytes<const N: usize, T> {
    pub align: [T; 0],
    pub bytes: [u8; N]
}

#[derive(Clone, Copy)]
pub struct U64Array<'data, const N: usize> {
    pub bytes: &'data [u8; N]
    
}

impl<const N: usize> U64Array<'_, N> {
    pub const LEN: usize = {
        [(); 0][(N % mem::size_of::<u64>() != 0) as usize];

        N / mem::size_of::<u64>()
    };
    
    pub fn get(&self, index: usize) -> Option<u64> {
        let size = mem::size_of::<u64>();
        let index = index * size;

        if N >= index + size {
            let buf = self.bytes[index..][..size].try_into().unwrap();
            Some(u64::from_le_bytes(buf))
        } else {
            None
        }
    }
}
