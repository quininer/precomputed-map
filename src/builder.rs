#![allow(clippy::uninlined_format_args)]

#[cfg(test)]
mod tests;
mod build;

use std::{ fs, cmp, fmt };
use std::io::{ self, Write };
use std::path::PathBuf;


/// Static Map builder
///
/// Computes an appropriate static map based on the provided keys.
pub struct MapBuilder<'a, K> {
    seed: Option<u64>,
    limit: Option<u64>,
    ord: Option<OrdFunc<'a, K>>,
    hash: Option<HashFunc<'a, K>>,
    next_seed: fn(u64, u64) -> u64,
}

pub type OrdFunc<'a, K> = &'a dyn Fn(&K, &K) -> cmp::Ordering;
pub type HashFunc<'a, K> = &'a dyn Fn(u64, &K) -> u64;

impl<'a, K> Default for MapBuilder<'a, K> {
    fn default() -> Self {
        MapBuilder::new()
    }
}

impl<'a, K> MapBuilder<'a, K> {
    pub fn new() -> Self {
        MapBuilder {
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

    /// Creates a Map with the specified keys
    ///
    /// # NOTE
    ///
    /// Note that the keys used must be unique, otherwise the build will not succeed.
    pub fn build(&self, keys: &[K]) -> Result<MapOutput, BuildFailed> {
        if keys.len() <= 16 {
            // For tiny amounts of data, binary search is usually faster.
            //
            // At most 4 comparisons will be faster than a high-quality hash.
            if let Some(output) = build::build_tiny(self, keys) {
                return Ok(output);
            }
        }

        if keys.len() <= 128 {
            // For small numbers of keys, try to build the smallest and fastest phf.
            //
            // This outperforms all other phfs,
            // but for large numbers of keys, this may not be able to find the seed in a reasonable time.
            //
            // If the keys length is greater than 12, it will usually fallback to medium map.
            if let Some(output) = build::build_small(self, keys) {
                return Ok(output);
            }
        }

        if keys.len() > 10 * 1024 * 1024 {
            return Err(BuildFailed("WARN: \
                We currently don't have good support for large numbers of keys,\
                and this construction may be slow or not complete in a reasonable time.\
            "));
        }

        // A typical PHF, but not optimized for construction time, and no sharding.
        // 
        // It is suitable for large amounts of data that need to be embedded in a binary file,
        // but for data larger than that it is better to use a specialized PHF library.
        build::build_medium(self, keys)
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

pub struct ReferenceId(usize);

#[derive(Debug)]
pub struct MapOutput {
    pub kind: MapKind,
    pub index: Box<[usize]>
}

/// Code Generator
///
/// Generate code based on the constructed Map and the provided sequence.
pub struct CodeBuilder {
    name: String,
    hash: String,
    dir: PathBuf,
    vis: Option<String>,
    list: Vec<OutputEntry>,
    bytes_writer: Option<CountWriter<fs::File>>,
    u32seq_writer: Option<CountWriter<fs::File>>,
}

struct OutputEntry {
    name: Option<String>,
    kind: OutputKind
}

enum OutputKind {
    Custom {
        name: String,
    },
    U8Seq {
        offset: usize,
        len: usize
    },
    BytesSeq {
        offset: usize,
        len: usize,
        index: ReferenceId
    },
    U32Seq {
        offset: usize,
        len: usize
    },
    List {
        item_type: String,
        value: String,
        len: usize,
        searchable: bool
    },
    Pair {
        keys: ReferenceId,
        values: ReferenceId
    },
    Tiny(ReferenceId),
    Small {
        seed: u64,
        data: ReferenceId
    },
    Medium {
        seed: u64,
        slots: u32,
        pilots: ReferenceId,
        remap: ReferenceId,
        data: ReferenceId
    }
}

struct CountWriter<W> {
    writer: W,
    count: usize
}

impl MapOutput {
    /// The seed can be saved and used in next compute to keep output stable.
    pub fn seed(&self) -> Option<u64> {
        match &self.kind {
            MapKind::Tiny => None,
            MapKind::Small(seed) => Some(*seed),
            MapKind::Medium { seed, .. } => Some(*seed)
        }
    }

    /// Generates a reordered iterator based on the constructed map.
    ///
    /// The lengths of provided lists must be equal.    
    pub fn reorder<'list: 'map, 'map, T>(&'map self, list: &'list [T])
        -> impl ExactSizeIterator<Item = &'list T> + DoubleEndedIterator + 'map
    {
        assert_eq!(self.index.len(), list.len());

        self.index.iter().map(|&idx| &list[idx])
    }

    /// Create static map
    ///
    /// # NOTE
    ///
    /// The provided data must be reordered, otherwise the behavior will be unexpected.
    pub fn create_map(&self, name: String, data: ReferenceId, builder: &mut CodeBuilder)
        -> io::Result<ReferenceId>
    {
        match &self.kind {
            MapKind::Tiny => {
                let id = builder.list.len();
                builder.list.push(OutputEntry {
                    name: Some(name),
                    kind: OutputKind::Tiny(data)
                });
                Ok(ReferenceId(id))
            },
            MapKind::Small(seed) => {
                let id = builder.list.len();
                builder.list.push(OutputEntry {
                    name: Some(name),
                    kind: OutputKind::Small { seed: *seed, data }
                });
                Ok(ReferenceId(id))                
            },
            MapKind::Medium { seed, slots, pilots, remap } => {
                let pilots = if pilots.len() > 1024 {
                    let writer = builder.bytes_writer()?;
                    let offset = writer.count;
                    writer.write_all(pilots)?;
                    let len = writer.count - offset;

                    let id = builder.list.len();
                    builder.list.push(OutputEntry {
                        name: None,
                        kind: OutputKind::U8Seq { offset, len }
                    });
                    ReferenceId(id)
                } else {
                    builder.create_list_raw(None, "u8".into(), false, pilots.iter().copied())?
                };

                let remap = builder.create_u32_seq_raw(None, remap.iter().copied())?;

                let id = builder.list.len();
                builder.list.push(OutputEntry {
                    name: Some(name),
                    kind: OutputKind::Medium {
                        seed: *seed,
                        slots: *slots,
                        pilots, remap, data
                    }
                });
                Ok(ReferenceId(id))
            },
        }
    }
}

impl CodeBuilder {
    /// Specifies the name, hash, and directory to use for the output map code.
    ///
    /// Note that `hash` must be a fully qualified type path that implements
    /// the [`HashOne`](crate::phf::HashOne) trait
    /// and is consistent with the algorithm used by MapBuilder.
    pub fn new(
        name: String,
        hash: String,
        dir: PathBuf,
    ) -> CodeBuilder {
        CodeBuilder {
            name, hash, dir,
            vis: None,
            list: Vec::new(),
            bytes_writer: None,
            u32seq_writer: None,
        }
    }

    /// This will configure the generated code as `pub(vis)`.
    pub fn set_visibility(&mut self, vis: Option<String>) {
        self.vis = vis;
    }

    fn bytes_writer(&mut self) -> io::Result<&mut CountWriter<fs::File>> {
        if self.bytes_writer.is_some() {
            Ok(self.bytes_writer.as_mut().unwrap())
        } else {
            let path = self.dir.join(format!("{}.bytes", self.name));
            let fd = fs::File::create(path)?;
            Ok(self.bytes_writer.get_or_insert(CountWriter {
                writer: fd,
                count: 0
            }))
        }
    }

    fn u32seq_writer(&mut self) -> io::Result<&mut CountWriter<fs::File>> {
        if self.u32seq_writer.is_some() {
            Ok(self.u32seq_writer.as_mut().unwrap())
        } else {
            let path = self.dir.join(format!("{}.u32seq", self.name));
            let fd = fs::File::create(path)?;
            Ok(self.u32seq_writer.get_or_insert(CountWriter {
                writer: fd,
                count: 0
            }))
        }
    }

    pub fn create_custom(&mut self, name: String) -> ReferenceId {
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: None,
            kind: OutputKind::Custom { name }
        });
        ReferenceId(id)
    }

    fn create_list_raw<SEQ, T>(
        &mut self,
        name: Option<String>,
        item_type: String,
        searchable: bool,
        seq: SEQ
    )
        -> io::Result<ReferenceId>
    where
        SEQ: ExactSizeIterator<Item = T>,
        T: fmt::Display
    {
        use std::io::Write;
        
        let len = seq.len();        
        let mut s = Vec::new();
        write!(s, "&[")?;
        for t in seq {
            write!(s, "{},", t)?;
        }
        write!(s, "]")?;
        let value = String::from_utf8(s).unwrap();
        
        let id = self.list.len();
        self.list.push(OutputEntry {
            name,
            kind: OutputKind::List { item_type, len, value, searchable }
        });
        Ok(ReferenceId(id))
    }

    pub fn create_list<SEQ, T>(&mut self, name: String, item_type: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = T> + ExactSizeIterator,
        T: fmt::Display
    {
        self.create_list_raw(Some(name), item_type, false, seq)
    }
    
    pub fn create_pair(&mut self, keys: ReferenceId, values: ReferenceId) -> ReferenceId {
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: None,
            kind: OutputKind::Pair { keys, values }
        });
        ReferenceId(id)
    }

    pub fn create_bytes_seq<SEQ, B>(&mut self, name: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = B> + ExactSizeIterator,
        B: AsRef<[u8]>
    {
        if seq.len() > 128 {
            let writer = self.bytes_writer()?;
            let offset = writer.count;
            let mut count = 0;
            let mut list = Vec::new();
            for buf in seq {
                let buf = buf.as_ref();
                writer.write_all(buf)?;

                let len: u32 = buf.len().try_into().unwrap();
                count += len;
                list.push(count);
            }
            let len = writer.count - offset;
            let index = self.create_u32_seq_raw(None, list.iter().copied())?;

            let id = self.list.len();
            self.list.push(OutputEntry {
                name: Some(name),
                kind: OutputKind::BytesSeq { offset, len, index }
            });
            Ok(ReferenceId(id))
        } else {
            self.create_list_raw(Some(name), "&'static [u8]".into(), true, seq.map(|b| format!("&{:?}", b.as_ref())))
        }
    }

    fn create_u32_seq_raw<SEQ>(&mut self, name: Option<String>, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = u32> + ExactSizeIterator
    {
        if seq.len() > 1024 {
            let writer = self.u32seq_writer()?;
            let offset = writer.count;
            for n in seq {
                writer.write_all(&n.to_le_bytes())?;
            }
            let len = writer.count - offset;

            let id = self.list.len();
            self.list.push(OutputEntry {
                name,
                kind: OutputKind::U32Seq { offset, len }
            });
            Ok(ReferenceId(id))
        } else {
            self.create_list_raw(name, "u32".into(), false, seq)
        }        
    }    

    pub fn create_u32_seq<SEQ>(&mut self, name: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = u32> + ExactSizeIterator
    {
        self.create_u32_seq_raw(Some(name), seq)
    }

    pub fn write_to(self, writer: &mut dyn io::Write) -> io::Result<()> {
        struct ReferenceEntry {
            name: String,
        }

        let crate_name = env!("CARGO_CRATE_NAME");
        let vis = self.vis.as_deref()
            .map(|vis| format!("pub({}) ", vis))
            .unwrap_or_default();
        let bytes_name = format!("PrecomputedBytes{}", self.name);
        let u32seq_name = format!("PrecomputedU32SeqBytes{}", self.name);        

        let bytes_count = self.bytes_writer.as_ref()
            .map(|writer| writer.count)
            .unwrap_or_default();
        if bytes_count != 0 {
            writeln!(writer,
                r#"{crate_name}::define!(const {bytes_name}: &[u8; {count}] = include "{file}.bytes");"#,
                count = bytes_count,
                file = self.name
            )?;
        }

        let u32seq_count = self.u32seq_writer.as_ref()
            .map(|writer| writer.count)
            .unwrap_or_default();
        if u32seq_count != 0 {
            writeln!(writer,
                r#"{crate_name}::define!(const {u32seq_name}: &[u8 align u32; {count}] = include "{file}.u32seq");"#,
                count = u32seq_count,
                file = self.name
            )?;
        }

        let mut list: Vec<ReferenceEntry> = Vec::with_capacity(self.list.len());

        for (idx, entry) in self.list.iter().enumerate() {
            let entry = match &entry.kind {
                OutputKind::Custom { name } => ReferenceEntry {
                    name: name.clone(),
                },
                OutputKind::U8Seq { offset, len } => {
                    let ty = format!(
                        "{crate_name}::store::SliceData<{}, {}, {}>",
                        offset,
                        len,
                        bytes_name,
                    );

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "{vis}type {} = {};", entry_name, ty)?;
                        ReferenceEntry { name: entry_name.clone() }
                    } else {
                        ReferenceEntry { name: ty }
                    }
                },
                OutputKind::U32Seq { offset, len } => {
                    let data_ty = format!(
                        "{crate_name}::store::SliceData<{}, {}, {}>",
                        offset,
                        len,
                        u32seq_name,
                    );
                    let ty = format!(
                        "{crate_name}::aligned::AlignedArray<{}, u32, {}>",
                        len,
                        data_ty,
                    );

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "{vis}type {} = {};", entry_name, ty)?;
                        ReferenceEntry { name: entry_name.clone() }
                    } else {
                        ReferenceEntry { name: ty }
                    }
                },
                OutputKind::BytesSeq { offset, len, index } => {
                    let data_ty = format!(
                        "{crate_name}::store::SliceData<{}, {}, {}>",
                        offset, len, bytes_name
                    );
                    let ty = format!(
                        "{crate_name}::seq::CompactSeq<{}, {}>",
                        &list[index.0].name,
                        data_ty,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}type {} = {};", entry_name, ty)?;
                    ReferenceEntry { name: entry_name.clone() }                    
                },
                OutputKind::List { item_type, value, len, searchable } => {
                    let namebuf;
                    let entry_name = if let Some(name) = entry.name.as_ref() {
                        name
                    } else {
                        namebuf = format!("PrecomputedList{}{}", self.name, idx);
                        &namebuf
                    };
                    writeln!(
                        writer,
                        "{crate_name}::define!(const {}{}: &[{}; {}] = {});",
                        searchable.then_some("searchable ").unwrap_or_default(),
                        entry_name,
                        item_type,
                        len,
                        value
                    )?;
                    ReferenceEntry { name: entry_name.clone() }
                },
                OutputKind::Pair { keys, values } => {
                    let ty = format!(
                        "({}, {})",
                        &list[keys.0].name,
                        &list[values.0].name,
                    );
                    ReferenceEntry { name: ty }                    
                }
                OutputKind::Tiny(data) => {
                    let ty = format!(
                        "{crate_name}::TinyMap<{}>",
                        &list[data.0].name
                    );
                    let val = format!("{crate_name}::TinyMap::new()");

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { name: entry_name.clone() }
                },
                OutputKind::Small { seed, data } => {
                    let ty = format!(
                        "{crate_name}::SmallMap<{}, {}>",
                        &list[data.0].name,
                        self.hash,
                    );
                    let val = format!(
                        "{crate_name}::SmallMap::new({})",
                        seed,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { name: entry_name.clone() }
                },
                OutputKind::Medium { seed, slots, pilots, remap, data } => {
                    let ty = format!(
                        "{crate_name}::MediumMap<{}, {}, {}, {}, {}>",
                        slots,
                        &list[pilots.0].name,
                        &list[remap.0].name,
                        &list[data.0].name,
                        self.hash,
                    );
                    let val = format!(
                        "{crate_name}::MediumMap::new({})",
                        seed,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { name: entry_name.clone() }
                },
            };

            list.push(entry);
        }

        Ok(())
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
