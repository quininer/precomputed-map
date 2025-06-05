use std::{ cmp, iter };
use std::hash::{ Hash, Hasher };
use crate::phf;


pub struct MapBuilder<'a, K> {
    keys: &'a [K],
    seed: Option<u64>,
    max_search_limit: Option<usize>,
    ord: Option<&'a dyn Fn(&K, &K) -> cmp::Ordering>,
    hash: Option<&'a dyn Fn(u64, &dyn DynHash) -> u64>,
}

pub trait DynHash {
    fn hash(&self, hasher: &mut dyn Hasher);
}

impl<H: Hash> DynHash for H {
    fn hash(&self, mut hasher: &mut dyn Hasher) {
        Hash::hash(self, &mut hasher);
    }
}

impl<'a, K: Hash> MapBuilder<'a, K> {
    pub fn new(data: &'a [K]) -> Self {
        MapBuilder {
            keys: data,
            max_search_limit: None,
            seed: None,
            ord: None,
            hash: None,
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

    pub fn set_ord(&mut self, f: &'a impl Fn(&K, &K) -> cmp::Ordering) -> &mut Self {
        self.ord = Some(f);
        self
    }

    pub fn set_hash(&mut self, f: &'a impl Fn(u64, &dyn DynHash) -> u64) -> &mut Self {
        self.hash = Some(f);
        self
    }

    pub fn build(&self) -> MapOutput {
        use crate::fast_reduct64;
        
        // build tiny
        if self.keys.len() <= 16
            && let Some(ord) = self.ord.as_ref()
        {
            let mut index = (0..self.keys.len()).collect::<Box<[_]>>();
            index.sort_by(|&x, &y| ord(&self.keys[x], &self.keys[y]));

            return MapOutput {
                kind: MapKind::Tiny,
                index
            };
        }

        // random seed
        let init_seed = self.seed.unwrap_or_else(|| {
            use std::hash::BuildHasher;
            
            std::collections::hash_map::RandomState::new().hash_one(0x42)
        });
        let mut seed = init_seed;

        // build small
        if self.keys.len() <= 1024
            && let Some(hash) = self.hash.as_ref()
        {
            let mut hashes = Vec::with_capacity(self.keys.len());
            let mut map = vec![None; self.keys.len()];
            let keys_len: u32 = self.keys.len().try_into().unwrap();

            'search: for c in 0..1024 {
                map.iter_mut().for_each(|idx| *idx = None);
                hashes.clear();
                hashes.extend(self.keys.iter().map(|v| hash(seed, v)));

                for (idx, &v) in hashes.iter().enumerate() {
                    let new_idx = fast_reduct64(v, keys_len.into()) as usize;

                    if map[new_idx].replace(idx).is_some() {
                        seed = (init_seed >> 32) ^ hash(init_seed, &c as &dyn DynHash);
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

        if self.keys.len() > 10 * 1024 * 1024 {
            eprintln!("WARN: \
                We currently don't have good support for large numbers of keys,\
                and this construction may be slow or not complete in a reasonable time.\
            ");
        }

        // build medium
        if let Some(hash) = self.hash.as_ref() {
            #[derive(Default)]
            struct Bucket {
                slots: Vec<usize>
            }

            struct Slot {
                bucket: u32,
                keys_idx: usize,
            }
            
            let alpha = 0.99;
            let lambda = 3.0;

            let keys_len: u32 = self.keys.len().try_into().unwrap();
            let slots_len = {
                let len = (f64::from(keys_len) / alpha).ceil() as u32;

                // Avoid powers of two, since then %S does not depend on all bits.
                len + (len.is_power_of_two() as u32)
            };
            let buckets_len = {
                let len = (f64::from(keys_len) / lambda).ceil() as u32;

                // Add a few extra buckets to avoid collisions for small n.
                len + 3
            };

            let mut buckets = (0..buckets_len)
                .map(|_| Bucket::default())
                .collect::<Box<[_]>>();
            let mut pilots = vec![0; buckets_len as usize].into_boxed_slice();
            let mut order = (0..buckets_len).collect::<Box<_>>();
            let mut slots = (0..slots_len).map(|_| None).collect::<Box<[_]>>();
            let mut hashes = Vec::with_capacity(self.keys.len());
            let mut stack = Vec::new();
            let mut values_to_add = Vec::new();
            let mut recent = Vec::new();

            'search: for c in 0.. {
                buckets.iter_mut().for_each(|bucket| bucket.slots.clear());
                pilots.iter_mut().for_each(|p| *p = 0);
                recent.clear();

                hashes.clear();
                hashes.extend(self.keys.iter().map(|key| hash(seed, key)));

                for (idx, &v) in hashes.iter().enumerate() {
                    let bucket_idx = fast_reduct64(v, buckets_len.into()) as usize;
                    buckets[bucket_idx].slots.push(idx);
                }

                order.sort_by_key(|&bucket_idx| cmp::Reverse(buckets[bucket_idx as usize].slots.len()));

                for &new_bucket_idx in &order {
                    if buckets[new_bucket_idx as usize].slots.is_empty() {
                        pilots[new_bucket_idx as usize] = 0;
                        continue
                    }
                    
                    stack.clear();
                    stack.push(new_bucket_idx);

                    'bucket: while let Some(bucket_idx) = stack.pop() {
                        // Do not evict buckets that have already been evicted.
                        //
                        // this is simpler than the original ptr-hash code, but can completely prevent cycles.
                        recent.push(bucket_idx);

                        // fast search pilot
                        'pilot: for p in 0..=u8::MAX {
                            values_to_add.clear();

                            let hp = phf::hash_pilot(seed, p);

                            for (keys_idx, slot_idx) in buckets[bucket_idx as usize]
                                .slots
                                .iter()
                                .map(|&keys_idx| (keys_idx, fast_reduct64(hashes[keys_idx] ^ hp, slots_len.into())))
                            {
                                if slots[slot_idx as usize].is_some()
                                    || values_to_add.iter().any(|(_, prev)| *prev == slot_idx)
                                {
                                    continue 'pilot
                                }

                                values_to_add.push((keys_idx, slot_idx));
                            }

                            pilots[bucket_idx as usize] = p;

                            for &(keys_idx, slot_idx) in &values_to_add {
                                slots[slot_idx as usize] = Some(Slot {
                                    bucket: bucket_idx,
                                    keys_idx 
                                });
                            }

                            continue 'bucket
                        }

                        // search best pilot (minimal collisions)
                        let mut best = None;

                        'pilot: for p in 0..=u8::MAX {
                            values_to_add.clear();

                            // start from a slightly different point, just 42 because we don't like random.
                            let p = p.wrapping_add(0x42);
                            let hp = phf::hash_pilot(seed, p);
                            let mut collision_score = 0;

                            for (keys_idx, slot_idx) in buckets[bucket_idx as usize].slots
                                .iter()
                                .map(|&keys_idx| (keys_idx, fast_reduct64(hashes[keys_idx] ^ hp, slots_len.into())))
                            {
                                let new_score = match slots[slot_idx as usize].as_ref() {
                                    None => 0,
                                    Some(slot) if
                                        values_to_add.iter().any(|(_, prev)| *prev == slot_idx) 
                                        || recent.contains(&slot.bucket)
                                        => continue 'pilot,
                                    Some(slot) => {
                                        buckets[slot.bucket as usize].slots.len().pow(2)
                                    },
                                };

                                values_to_add.push((keys_idx, slot_idx));

                                collision_score += new_score;

                                if let Some((best_score, _)) = best
                                    && new_score != 0
                                    && collision_score > best_score
                                {
                                    continue 'pilot
                                }
                            }

                            best = Some((collision_score, p));

                            // Since we already checked for a collision-free solution,
                            // the next best is a single collision of size b_len.
                            if collision_score == buckets[new_bucket_idx as usize].slots.len().pow(2) {
                                break
                            }
                        }

                        let Some((_, p)) = best else {
                            // No available pilot was found, so this seed is abandoned.
                            seed = (init_seed >> 32) ^ hash(init_seed, &c as &dyn DynHash);
                            continue 'search
                        };

                        pilots[bucket_idx as usize] = p;
                        let hp = phf::hash_pilot(seed, p);

                        for (keys_idx, slot_idx) in buckets[bucket_idx as usize].slots
                            .iter()
                            .map(|&keys_idx| (keys_idx, fast_reduct64(hashes[keys_idx] ^ hp, slots_len.into())))
                        {
                            if let Some(old_slot) = slots[slot_idx as usize].replace(Slot {
                                bucket: bucket_idx,
                                keys_idx
                            }) {
                                // Eviction conflict bucket
                                stack.push(old_slot.bucket);
                            }
                        }
                    }
                }
            }
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
