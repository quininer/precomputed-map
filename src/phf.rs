use core::hash::{ Hash, Hasher };
use core::marker::PhantomData;

pub trait HashOne {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64;
}

pub fn split_key(hash: u128) -> (u32, u32, u32) {
    let h1 = (hash >> 64) as u32;
    let h2 = hash as u64;
    let g = (h2 >> 32) as u32;
    let h2 = h2 as u32;    
    (g, h1, h2)
}

pub fn hash_pilot(k: u64, pilot: u8) -> u64 {
    const C: u64 = 0x517cc1b727220a95;

    // fxhash
    C.wrapping_mul(k ^ u64::from(pilot))
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
