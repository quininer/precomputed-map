#![allow(clippy::uninlined_format_args)]

#[cfg(test)]
mod tests;
mod build;
mod codegen;

use std::{ cmp, fmt };
pub use codegen::*;


/// Static Map builder
///
/// Computes an appropriate static map based on the provided keys.
pub struct MapBuilder<'a, K> {
    keys: &'a [K],
    seed: Option<u64>,
    limit: Option<u64>,
    ord: Option<OrdFunc<'a, K>>,
    hash: Option<HashFunc<'a, K>>,
    next_seed: fn(u64, u64) -> u64,
}

pub type OrdFunc<'a, K> = &'a dyn Fn(&K, &K) -> cmp::Ordering;
pub type HashFunc<'a, K> = &'a dyn Fn(u64, &K) -> u64;

impl<'a, K> MapBuilder<'a, K> {
    pub fn new(keys: &'a [K]) -> Self {
        MapBuilder {
            keys,
            limit: None,
            seed: None,
            ord: None,
            hash: None,
            next_seed: |init_seed, c| {
                use std::hash::Hasher;

                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                hasher.write_u64(init_seed);
                hasher.write_u64(c);
                hasher.finish()
            },
        }
    }

    pub fn set_limit(&mut self, limit: Option<u64>) -> &mut Self {
        self.limit = limit;
        self
    }

    pub fn set_seed(&mut self, seed: u64) -> &mut Self {
        self.seed = Some(seed);
        self
    }

    pub fn set_ord(&mut self, f: OrdFunc<'a, K>) -> &mut Self {
        self.ord = Some(f);
        self
    }

    pub fn set_hash(&mut self, f: HashFunc<'a, K>) -> &mut Self {
        self.hash = Some(f);
        self
    }

    pub fn set_next_seed(&mut self, f: fn(u64, u64) -> u64)
        -> &mut Self
    {
        self.next_seed = f;
        self
    }

    pub fn build(&self) -> Result<MapOutput, BuildFailed> {
        if self.keys.len() <= 16 {
            // For tiny amounts of data, binary search is usually faster.
            //
            // At most 4 comparisons will be faster than a high-quality hash.
            if let Some(output) = build::build_tiny(self) {
                return Ok(output);
            }
        }

        if self.keys.len() <= 128 {
            // For small numbers of keys, try to build the smallest and fastest phf.
            //
            // This outperforms all other phfs,
            // but for large numbers of keys, this may not be able to find the seed in a reasonable time.
            //
            // If the keys length is greater than 12, it will usually fallback to medium map.
            if let Some(output) = build::build_small(self) {
                return Ok(output);
            }
        }

        if self.keys.len() > 10 * 1024 * 1024 {
            return Err(BuildFailed("WARN: \
                We currently don't have good support for large numbers of keys,\
                and this construction may be slow or not complete in a reasonable time.\
            "));
        }

        // A typical PHF, but not optimized for construction time, and no sharding.
        // 
        // It is suitable for large amounts of data that need to be embedded in a binary file,
        // but for data larger than that it is better to use a specialized PHF library.
        build::build_medium(self)
    }
}

#[derive(Debug)]
pub struct BuildFailed(&'static str);

#[derive(Debug)]
pub enum MapKind {
    Tiny,
    Small(u64),
    Medium {
        seed: u64,
        slots: u32,
        pilots: Box<[u8]>,
        remap: Box<[u32]>,
    }
}

impl fmt::Display for BuildFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for BuildFailed {}

#[derive(Debug)]
pub struct MapOutput {
    pub kind: MapKind,
    pub index: Box<[usize]>
}
