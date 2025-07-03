use std::hash::{ Hash, Hasher };
use precomputed_map::phf::HashOne;


pub type Default = precomputed_map::phf::U64Hasher<std::collections::hash_map::DefaultHasher>;

pub struct Sip;

impl HashOne for Sip {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        let mut hasher = siphasher::sip::SipHasher13::new_with_keys(k, 0);
        v.hash(&mut hasher);
        hasher.finish()        
    }
}

pub struct Xx3;

impl HashOne for Xx3 {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        let mut hasher = xxhash_rust::xxh3::Xxh3::with_seed(k);
        v.hash(&mut hasher);
        hasher.finish()
    }
}

pub struct Fx;

impl HashOne for Fx {
    // #[inline(never)]
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        // FIXME The platform-independent fxhash implementation should be used
        let mut hasher = rustc_hash::FxHasher::with_seed(k as usize);
        v.hash(&mut hasher);
        hasher.finish()
    }
}

pub struct Fold;

impl HashOne for Fold {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        let mut hasher = foldhash::fast::FoldHasher::with_seed(k, foldhash::SharedSeed::global_fixed());
        v.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(feature = "gxhash")]
pub struct Gx;

#[cfg(feature = "gxhash")]
impl HashOne for Gx {
    fn hash_one<T: Hash>(k: u64, v: T) -> u64 {
        let mut hasher = gxhash::GxHasher::with_seed(k);
        v.hash(&mut hasher);
        hasher.finish()
    }
}
