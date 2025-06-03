use core::hash::{ Hash, Hasher };
use core::marker::PhantomData;

pub trait HashOne {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64;
}

pub trait HashOne128 {
    fn hash_one128<T: Hash>(k: u64, v: T) -> u128;
}

pub fn displace(h1: u32, h2: u32, d0: u32, d1: u32) -> u32 {
    // Then, for each bucket Bi, 0 ≤ i < r,
    // we will assign a pair of displacements (d0, d1) so that each key x ∈ Bi is placed in an empty bin
    // given by (f1(x) + d0f2(x) + d1) mod m.

    h1.wrapping_add(d0.wrapping_mul(h2)).wrapping_add(d1)
}

#[derive(Default)]
pub struct U64Hasher<H: Hasher + Default>(PhantomData<H>);

impl<H: Hasher + Default> HashOne for U64Hasher<H> {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        let mut h = H::default();
        k.hash(&mut h);
        v.hash(&mut h);
        h.finish()
    }
}

impl<H: Hasher + Default> HashOne128 for U64Hasher<H> {
    fn hash_one128<T: Hash>(k: u64, v: T) -> u128 {
        let mut h1 = H::default();
        let mut h2 = H::default();
        h1.write_u64(k);
        h2.write_u64(k ^ 0x1);
        v.hash(&mut h1);
        v.hash(&mut h2);
        let h1 = h1.finish();
        let h2 = h2.finish();
        u128::from(h1 << 64) | u128::from(h2)
    }
}
