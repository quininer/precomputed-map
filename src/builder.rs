#[cfg(test)]
mod tests;
mod build;

use std::{ fs, cmp, fmt };
use std::io::{ self, Write };


pub struct MapBuilder<'a, K> {
    keys: &'a [K],
    seed: Option<u64>,
    limit: Option<u64>,
    ord: Option<&'a dyn Fn(&K, &K) -> cmp::Ordering>,
    hash: Option<&'a dyn Fn(u64, &K) -> u64>,
    next_seed: &'a dyn Fn(u64, u64) -> u64,
    force_build: bool,
}

impl<'a, K> MapBuilder<'a, K> {
    pub fn new(keys: &'a [K]) -> Self {
        MapBuilder {
            keys,
            limit: None,
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

    pub fn set_limit(&mut self, limit: Option<u64>) -> &mut Self {
        self.limit = limit;
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

pub struct OutputBuilder {
    name: String,
    hash: String,
    list: Vec<OutputEntry>,
    bytes_writer: CountWriter<fs::File>,
    u32seq_writer: CountWriter<fs::File>,
}

struct OutputEntry {
    name: Option<String>,
    kind: OutputKind
}

enum OutputKind {
    Custom {
        r#type: String,
        value: String,
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
        len: usize
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
    pub fn seed(&self) -> Option<u64> {
        match &self.kind {
            MapKind::Tiny => None,
            MapKind::Small(seed) => Some(*seed),
            MapKind::Medium { seed, .. } => Some(*seed)
        }
    }
    
    pub fn reorder<'a: 'i, 'i, T>(&'i self, list: &'a [T])
        -> impl Iterator<Item = &'a T> + 'i
    {
        assert_eq!(self.index.len(), list.len());

        self.index.iter().map(|&idx| &list[idx])
    }
    
    pub fn create_map(&self, name: String, data: ReferenceId, builder: &mut OutputBuilder)
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
                let pilots = {
                    let offset = builder.bytes_writer.count;
                    builder.bytes_writer.write_all(&pilots)?;
                    let len = builder.bytes_writer.count - offset;

                    let id = builder.list.len();
                    builder.list.push(OutputEntry {
                        name: None,
                        kind: OutputKind::U8Seq { offset, len }
                    });
                    ReferenceId(id)
                };

                let remap = {
                    let offset = builder.u32seq_writer.count;
                    for &n in remap {
                        builder.u32seq_writer.write_all(&n.to_le_bytes())?;
                    }
                    let len = builder.u32seq_writer.count - offset;

                    let id = builder.list.len();
                    builder.list.push(OutputEntry {
                        name: None,
                        kind: OutputKind::U32Seq { offset, len }
                    });
                    ReferenceId(id)                    
                };

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

impl OutputBuilder {
    pub fn new<'w>(
        name: String,
        hash: String,
        bytes_writer: fs::File,
        u32seq_writer: fs::File
    ) -> OutputBuilder {
        OutputBuilder {
            name, hash,
            list: Vec::new(),
            bytes_writer: CountWriter { writer: bytes_writer, count: 0 },
            u32seq_writer: CountWriter { writer: u32seq_writer, count: 0 },
        }
    }
    
    pub fn create_custom(&mut self, r#type: String, value: String) -> ReferenceId {
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: None,
            kind: OutputKind::Custom { r#type, value }
        });
        ReferenceId(id)
    }

    pub fn create_list<SEQ, T>(&mut self, name: String, item_type: String, seq: SEQ)
        -> Result<ReferenceId, fmt::Error>
    where
        SEQ: Iterator<Item = T> + ExactSizeIterator,
        T: fmt::Display
    {
        use fmt::Write;

        let len = seq.len();        
        let mut s = String::new();
        write!(s, "&[")?;
        for t in seq {
            write!(s, "{},", t)?;
        }
        write!(s, "]")?;
        
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: Some(name),
            kind: OutputKind::List { item_type, len, value: s }
        });
        Ok(ReferenceId(id))
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
        SEQ: Iterator<Item = B>,
        B: AsRef<[u8]>
    {
        let offset = self.bytes_writer.count;
        let mut count = 0;
        let mut list = Vec::new();
        for buf in seq {
            let buf = buf.as_ref();
            self.bytes_writer.write_all(buf)?;

            let len: u32 = buf.len().try_into().unwrap();
            count += len;
            list.push(count);
        }
        let len = self.bytes_writer.count - offset;
        let index = self.create_u32_seq_raw(None, list.iter().copied())?;

        let id = self.list.len();
        self.list.push(OutputEntry {
            name: Some(name),
            kind: OutputKind::BytesSeq { offset, len, index }
        });
        Ok(ReferenceId(id))
    }

    fn create_u32_seq_raw<SEQ>(&mut self, name: Option<String>, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = u32>
    {
        let offset = self.u32seq_writer.count;
        for n in seq {
            self.u32seq_writer.write_all(&n.to_le_bytes())?;
        }
        let len = self.u32seq_writer.count - offset;

        let id = self.list.len();
        self.list.push(OutputEntry {
            name,
            kind: OutputKind::U32Seq { offset, len }
        });
        Ok(ReferenceId(id))        
    }    

    pub fn create_u32_seq<SEQ>(&mut self, name: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = u32>
    {
        self.create_u32_seq_raw(Some(name), seq)
    }

    pub fn build(self, writer: &mut dyn io::Write) -> io::Result<()> {
        struct ReferenceEntry {
            r#type: String,
            value: String
        }

        if self.bytes_writer.count != 0 {
            writeln!(writer,
                r#"const {name}_BYTES: &[u8; {count}] = include_bytes!("{file}.bytes");"#,
                count = self.bytes_writer.count,
                name = self.name.to_ascii_uppercase(),
                file = self.name
            )?;
        }

        if self.u32seq_writer.count != 0 {
            writeln!(writer,
                "\
                const {name}_U32SEQ: &[u8; {count}] = {{
                    static {name}_BYTES_U32SEQ: static_datamap::aligned::AlignedBytes<{count}, u32> = \
                        static_datamap::aligned::AlignedBytes {{
                            align: [],
                            bytes: *include_bytes!(\"{file}.u32seq\")
                        }};

                    &{name}_BYTES_U32SEQ.bytes
                }};\
                ",
                count = self.u32seq_writer.count,
                name = self.name.to_ascii_uppercase(),
                file = self.name
            )?;
        }

        let bytes = format!("{}_BYTES", self.name.to_ascii_uppercase());
        let u32seq = format!("{}_U32SEQ", self.name.to_ascii_uppercase());
        let mut list: Vec<ReferenceEntry> = Vec::with_capacity(self.list.len());

        for entry in &self.list {
            let entry = match &entry.kind {
                OutputKind::Custom { r#type, value } => ReferenceEntry {
                    r#type: r#type.clone(),
                    value: value.clone()
                },
                OutputKind::U8Seq { offset, len } => {
                    let ty = format!(
                        "static_datamap::store::ConstSlice<'static, {}, {}, {}>",
                        self.bytes_writer.count,
                        offset,
                        len
                    );
                    let val = format!("<{}>::new({})", ty, bytes);

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "const {}: {} = {};", entry_name, ty, val)?;
                        ReferenceEntry { r#type: ty, value: entry_name.clone() }
                    } else {
                        ReferenceEntry { r#type: ty, value: val }
                    }
                },
                OutputKind::U32Seq { offset, len } => {
                    let data_ty = format!(
                        "static_datamap::store::ConstSlice<'static, {}, {}, {}>",
                        self.u32seq_writer.count,
                        offset,
                        len
                    );
                    let ty = format!(
                        "static_datamap::aligned::AlignedArray<{}, u32, {}>",
                        len,
                        data_ty
                    );
                    let data_val = format!("<{}>::new({})", data_ty, u32seq);
                    let val = format!("<{}>::new({})", ty, data_val);

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "const {}: {} = {};", entry_name, ty, val)?;
                        ReferenceEntry { r#type: ty, value: entry_name.clone() }
                    } else {
                        ReferenceEntry { r#type: ty, value: val }
                    }
                },
                OutputKind::BytesSeq { offset, len, index } => {
                    let ty = format!(
                        "static_datamap::seq::CompactSeq<'static, {}, {}, {}, {}>",
                        self.bytes_writer.count,
                        offset,
                        len,
                        &list[index.0].r#type
                    );
                    let val = format!(
                        "static_datamap::seq::CompactSeq::new({}, static_datamap::store::ConstSlice::new({}))",
                        &list[index.0].value,
                        bytes
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }                    
                },
                OutputKind::List { item_type, value, len } => {
                    let ty = format!("static_datamap::seq::List<'static, {}, {}>", len, item_type);
                    let val = format!("static_datamap::seq::List({})", value);
                    
                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "const {}: {} = {}", entry_name, ty, val)?;
                        ReferenceEntry { r#type: ty, value: entry_name.clone() }
                    } else {
                        ReferenceEntry { r#type: ty, value: val }
                    }
                },
                OutputKind::Pair { keys, values } => {
                    let ty = format!(
                        "({}, {})",
                        &list[keys.0].r#type,
                        &list[values.0].r#type,
                    );
                    let val = format!(
                        "({}, {})",
                        &list[keys.0].value,
                        &list[values.0].value,
                    );
                    
                    ReferenceEntry { r#type: ty, value: val }                    
                }
                OutputKind::Tiny(data) => {
                    let ty = format!(
                        "static_datamap::TinyMap<'static, {}>",
                        &list[data.0].r#type
                    );
                    let val = format!(
                        "<{}>::new({})",
                        ty,
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "static {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }
                },
                OutputKind::Small { seed, data } => {
                    let ty = format!(
                        "static_datamap::SmallMap<'static, {}, {}>",
                        &list[data.0].r#type,
                        self.hash,
                    );
                    let val = format!(
                        "<{}>::new({}, {})",
                        ty,
                        seed,
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "static {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }
                },
                OutputKind::Medium { seed, slots, pilots, remap, data } => {
                    let ty = format!(
                        "static_datamap::MediumMap<'static, {}, {}, {}, {}>",
                        &list[pilots.0].r#type,
                        &list[remap.0].r#type,
                        &list[data.0].r#type,
                        self.hash,
                    );
                    let val = format!(
                        "static_datamap::MediumMap::new({}, {}, {}, {}, {})",
                        seed,
                        slots,
                        &list[pilots.0].value,
                        &list[remap.0].value,
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "static {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }
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
