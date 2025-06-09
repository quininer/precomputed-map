#[cfg(test)]
mod tests;
mod build;

use std::{ cmp, fmt };
use std::io::{ self, Write };
use std::ops::Range;


pub struct MapBuilder<'a, K> {
    keys: &'a [K],
    seed: Option<u64>,
    max_search_limit: Option<u64>,
    ord: Option<&'a dyn Fn(&K, &K) -> cmp::Ordering>,
    hash: Option<&'a dyn Fn(u64, &K) -> u64>,
    next_seed: &'a dyn Fn(u64, u64) -> u64,
    force_build: bool,
}

impl<'a, K> MapBuilder<'a, K> {
    pub fn new(keys: &'a [K]) -> Self {
        MapBuilder {
            keys,
            max_search_limit: None,
            seed: None,
            ord: None,
            hash: None,
            next_seed: &|init_seed, c| {
                use std::hash::BuildHasher;
    
                std::collections::hash_map::RandomState::new()
                    .hash_one((init_seed, c))
            },
            force_build: false
        }
    }

    pub fn set_max_search_limit(&mut self, limit: Option<u64>) -> &mut Self {
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

    pub fn set_hash(&mut self, f: &'a impl Fn(u64, &K) -> u64) -> &mut Self {
        self.hash = Some(f);
        self
    }

    pub fn set_next_seed(&mut self, f: &'a impl Fn(u64, u64) -> u64) -> &mut Self {
        self.next_seed = f;
        self
    }

    pub fn set_force_build(&mut self, flag: bool) -> &mut Self {
        self.force_build = flag;
        self
    }

    pub fn build(&self) -> Result<MapOutput, BuildFailed> {
        if self.keys.len() <= 16 {
            if let Some(output) = build::build_tiny(self) {
                return Ok(output);
            }
        }

        if self.keys.len() <= 1024 {
            if let Some(output) = build::build_small(self) {
                return Ok(output);
            }
        }

        if !self.force_build && self.keys.len() > 10 * 1024 * 1024 {
            return Err(BuildFailed("WARN: \
                We currently don't have good support for large numbers of keys,\
                and this construction may be slow or not complete in a reasonable time.\
            "));
        }

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

pub struct SeqOutput {
    //
}

pub struct OutputBuilder<'w> {
    list: Vec<OutputEntry>,
    bytes_writer: CountWriter<&'w mut dyn io::Write>,
    u32seq_writer: CountWriter<&'w mut dyn io::Write>,
}

enum OutputEntry {
    Tiny,
    Small(u64),
    Medium {
        seed: u64,
        pilots: Range<usize>,
        remap: Range<usize>,
    }
}

struct CountWriter<W> {
    writer: W,
    count: usize
}

impl MapOutput {
    pub fn output(&self, builder: &mut OutputBuilder<'_>)
        -> io::Result<()>
    {
        todo!()
    //     match &self.kind {
    //         MapKind::Tiny => writeln!(writer.code,
    //             "pub static MAP: static_datamap::TinyMap<'static, {}> = static_datamap::TinyMap::new({});",
    //             config.data_type,
    //             config.data
    //         )?,
    //         MapKind::Small(seed) => writeln!(writer.code,
    //             "pub static MAP: static_datamap::SmallMap<'static, {}, {}> = \
    //             static_datamap::SmallMap::new({}, {});",
    //             config.data_type,
    //             config.hash_type,
    //             seed,
    //             config.data
    //         )?,
    //         MapKind::Medium { seed, pilots, remap } => {
    //             let pilots_start = writer.bytes.count;
    //             writer.bytes.write_all(&pilots)?;
    //             let pilots = pilots_start..writer.bytes.count;

    //             let remap_start = writer.u32seq.count;
    //             for &n in remap {
    //                 writer.u32seq.write_all(&n.to_le_bytes())?;
    //             }
    //             let remap = remap_start..writer.u32seq.count;
                
    //             writeln!(writer.code,
    //                 "pub static MAP: static_datamap::MediumMap<'static, {}, {}, {}, {}> = \
    //                 static_datamap::MediumMap::new({}, {}, {}, {});",
    //                 "pilots",
    //                 "remap",
    //                 config.data_type,
    //                 config.hash_type,
    //                 seed,
    //                 "pilots",
    //                 "remap",
    //                 config.data
    //             )?                
    //         },
    //     }

    //     Ok(())
    }
}

impl<W: io::Write> io::Write for CountWriter<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(b)?;
        self.count += n;
        Ok(n)
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let n = self.writer.write_vectored(bufs)?;
        self.count += n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
