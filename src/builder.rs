use std::{ cmp, iter };
use crate::chd;


pub struct MapBuilder<'a, T> {
    data: &'a [T],
    seed: Option<u64>,
    max_search_limit: Option<usize>,
    ord: Option<&'a dyn Fn(&T, &T) -> cmp::Ordering>,
    hash: Option<&'a dyn Fn(u64, &T) -> u64>,
    hash128: Option<&'a dyn Fn(u64, &T) -> u128>,
}

impl<'a, T> MapBuilder<'a, T> {
    pub fn new(data: &'a [T]) -> Self {
        MapBuilder {
            data,
            max_search_limit: None,
            seed: None,
            ord: None,
            hash: None,
            hash128: None,
        }
    }

    pub fn set_max_search_limit(&mut self, limit: Option<usize>) -> &mut Self {
        self.max_search_limit = limit;
        self
    }

    pub fn set_seed(&mut self, seed: u64) -> &mut Self {
        self.seed = Some(seed);
        self
    }

    pub fn set_ord(&mut self, f: &'a impl Fn(&T, &T) -> cmp::Ordering) -> &mut Self {
        self.ord = Some(f);
        self
    }

    pub fn set_hash(&mut self, f: &'a impl Fn(u64, &T) -> u64) -> &mut Self {
        self.hash = Some(f);
        self
    }

    pub fn set_hash128(&mut self, f: &'a impl Fn(u64, &T) -> u128) -> &mut Self {
        self.hash128 = Some(f);
        self
    }

    pub fn build(&self) -> MapOutput {
        use crate::fast_reduct32;
        
        // build tiny
        if self.data.len() <= 16
            && let Some(ord) = self.ord.as_ref()
        {
            let mut index = (0..self.data.len()).collect::<Box<[_]>>();
            index.sort_by(|&x, &y| ord(&self.data[x], &self.data[y]));

            return MapOutput {
                kind: MapKind::Tiny,
                index
            };
        }

        // random seed
        let mut seed = self.seed.unwrap_or_else(|| {
            use std::hash::BuildHasher;
            
            std::collections::hash_map::RandomState::new().hash_one(0x42)
        });

        // build small
        if self.data.len() <= 1024
            && let Some(hash) = self.hash.as_ref()
        {
            let mut hashes = Vec::with_capacity(self.data.len());
            let mut map = vec![None; self.data.len()];

            'search: for _ in 0..1024 {
                map.iter_mut().for_each(|idx| *idx = None);
                hashes.clear();
                hashes.extend(self.data.iter().map(|v| hash(seed, v)));

                for (idx, &hash) in hashes.iter().enumerate() {
                    let new_idx = fast_reduct32(hash as u32, map.len() as u32) as usize;

                    if map[new_idx as usize].replace(idx).is_some() {
                        seed = seed.wrapping_add(1);
                        continue 'search;
                    }
                }

                break
            }

            let map = map.into_iter().collect::<Option<Box<[usize]>>>();
            if let Some(index) = map {
                return MapOutput {
                    kind: MapKind::Small(seed),
                    index
                };
            }
        }

        // build medium
        //
        // https://github.com/rust-phf/rust-phf/blob/v0.11.0/phf_generator/src/lib.rs#L28
        if let Some(hash) = self.hash128.as_ref() {
            #[derive(Default)]
            struct Bucket {
                idx: usize,
                keys: Vec<usize>
            }
            
            let lambda = 5;
            
            let mut hashes = Vec::with_capacity(self.data.len());
            let mut buckets: Box<[Bucket]> = (0..((self.data.len() + lambda - 1) / lambda))
                .map(|_| Bucket::default())
                .collect();
            let mut map = vec![None; self.data.len()].into_boxed_slice();
            let mut try_map = vec![0; self.data.len()].into_boxed_slice();
            let mut disps = vec![0; buckets.len()].into_boxed_slice();
            let mut values_to_add = Vec::new();
            
            let data_len: u32 = self.data.len().try_into().expect("too large");
            let buckets_len: u32 = buckets.len().try_into().expect("too large");

            'search: for count in 0.. {
                if Some(count) > self.max_search_limit {
                    break
                }
                
                buckets.iter_mut()
                    .enumerate()
                    .for_each(|(idx, bucket)| {
                        bucket.idx = idx;
                        bucket.keys.clear();
                    });
                hashes.clear();
                hashes.extend(self.data.iter().map(|v| hash(seed, v)));
                map.iter_mut().for_each(|idx| *idx = None);
                try_map.iter_mut().for_each(|g| *g = 0);

                for (idx, &hash) in hashes.iter().enumerate() {
                    let (g, ..) = chd::split_key(hash);
                    let new_idx = fast_reduct32(g, buckets_len) as usize;
                    buckets[new_idx].keys.push(idx);
                }

                buckets.sort_by_key(|bucket| cmp::Reverse(bucket.keys.len()));

                let mut generation = 0u64;

                'buckets: for bucket in &buckets {
                    'disps: for (d0, d1) in (0..data_len)
                        .flat_map(|d0| iter::repeat(d0).zip(0..data_len))
                    {
                        values_to_add.clear();
                        generation += 1;

                        for &key in &bucket.keys {
                            let (_, h1, h2) = chd::split_key(hashes[key]);
                            let idx = chd::displace(h1, h2, d0, d1);
                            let idx = fast_reduct32(idx, data_len) as usize;

                            // conflict, try next disps
                            if map[idx].is_some() || try_map[idx] == generation {
                                continue 'disps;
                            }

                            try_map[idx] = generation;
                            values_to_add.push((idx, key));
                        }

                        disps[bucket.idx] = u64::from(d0 << 32) | u64::from(d1);

                        for &(idx, key) in &values_to_add {
                            map[idx] = Some(key);
                        }

                        // found, next bucket
                        continue 'buckets
                    }
                    
                    // Unable to find displacements for a bucket
                    //
                    // try new seed
                    seed = seed.wrapping_add(1);
                    continue 'search;
                }
            };

            let map = map.into_iter().collect::<Option<Box<[usize]>>>();
            let map = map.expect("Unable to find a usable seed");

            return MapOutput {
                kind: MapKind::Medium { seed, disps },
                index: map
            };
        }

        panic!("No build method available")
    }
}

pub struct MapOutput {
    kind: MapKind,
    index: Box<[usize]>
}

pub enum MapKind {
    Tiny,
    Small(u64),
    Medium {
        seed: u64,
        disps: Box<[u64]>
    }
}
