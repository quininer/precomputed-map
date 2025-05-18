pub struct EliasFano<'data> {
    low_bits: &'data [u16],
    high_bits: SelectBits<'data>,
}


pub struct SelectBits<'data> {
    bits: &'data [u64],
    blocks: &'data [u64],
    superblocks: &'data [u64],
}

impl<'data> SelectBits<'data> {
    fn select1(&self, idx: usize) -> u16 {
        todo!()
    }
}

impl<'data> EliasFano<'data> {
    fn get(&self, idx: usize) -> u32 {
        let high = self.high_bits.select1(idx);
        let lower = self.low_bits[idx];
        u32::from(high << 16) | u32::from(lower)
    }
}
