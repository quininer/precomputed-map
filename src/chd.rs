use core::hash::Hasher;

pub trait Hasher128: Hasher {
    fn with_seed(k: u64) -> Self;
    fn finish_u128(&self) -> u128;
}

pub fn displace(h1: u32, h2: u32, d0: u32, d1: u32) -> u32 {
    // Then, for each bucket Bi, 0 ≤ i < r,
    // we will assign a pair of displacements (d0, d1) so that each key x ∈ Bi is placed in an empty bin
    // given by (f1(x) + d0f2(x) + d1) mod m.

    h1.wrapping_add(d0.wrapping_mul(h2)).wrapping_add(d1)
}

pub struct DoubleHasher<H: Hasher>(H, H);

impl<H: Hasher> Hasher for DoubleHasher<H> {
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
        self.1.write(bytes);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

impl<H: Default + Hasher> Hasher128 for DoubleHasher<H> {
    fn with_seed(k: u64) -> Self {
        let mut h1 = H::default();
        let mut h2 = H::default();
        h1.write_u64(k);
        h2.write_u64(k ^ 0x1);
        DoubleHasher(h1, h2)
    }
    
    fn finish_u128(&self) -> u128 {
        let h1 = self.0.finish();
        let h2 = self.1.finish();
        u128::from(h1 << 64) | u128::from(h2)
    }
}
