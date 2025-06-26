use std::{ fs, fmt };
use std::io::{ self, Write };
use std::path::{ Path, PathBuf };
use std::borrow::Cow;
use std::collections::HashMap;
use crate::seq::LimitedStr;
use super::*;


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
    str_writer: Option<CountWriter<fs::File>>,
    u32seq_writer: Option<CountWriter<fs::File>>,
}

pub struct StringPool<'s> {
    name: String,
    refenerce: String,
    pool: String,
    map: HashMap<Cow<'s, str>, LimitedStr>,
}

impl<'s> StringPool<'s> {
    pub fn new(name: String, refenerce: String) -> StringPool<'s> {
        StringPool {
            name, refenerce,
            pool: String::new(),
            map: HashMap::new()
        }
    }

    fn reference(&self) -> String {
        self.refenerce.clone()
    }
    
    pub fn insert(&mut self, s: &'s str) -> LimitedStr {
        self.insert_cow(Cow::Borrowed(s))
    }

    pub fn insert_cow(&mut self, s: Cow<'s, str>) -> LimitedStr {
        *self.map.entry(s.clone()).or_insert_with(|| {
            let offset = self.pool.len();
            self.pool.push_str(&s);
            let len: u8 = (self.pool.len() - offset).try_into().unwrap();
            let offset: u32 = offset.try_into().unwrap();

            if offset > (1 << 24) {
                panic!("string too large");
            }

            LimitedStr(offset | (u32::from(len) << 24))
        })
    }

    pub fn get(&self, id: LimitedStr) -> &str {
        let offset = id.offset();
        let len = id.len();

        &self.pool[offset..][..len]
    }

    pub fn write_to(self, dir: &Path, writer: &mut dyn io::Write) -> io::Result<()> {
        let name = format!("{}.strpool", self.name);
        fs::write(dir.join(&name), self.pool.as_bytes())?;

        writeln!(writer,
            "pub(crate) static {}: &str = include_str!(\"{}\")",
            self.name.to_uppercase(),
            name            
        )
    }
}

pub struct ReferenceId(usize);

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
    StrSeq {
        offset: usize,
        len: usize,
        index: ReferenceId
    },
    LimitedStrSeq {
        pool: String,
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
                let pilots = if pilots.len() > (4 * 1024) {
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
                    builder.create_list_raw(None, "u8".into(), pilots.iter().copied())?
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
            str_writer: None,
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

    fn str_writer(&mut self) -> io::Result<&mut CountWriter<fs::File>> {
        if self.str_writer.is_some() {
            Ok(self.str_writer.as_mut().unwrap())
        } else {
            let path = self.dir.join(format!("{}.str", self.name));
            let fd = fs::File::create(path)?;
            Ok(self.str_writer.get_or_insert(CountWriter {
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

    pub fn create_custom(&mut self, r#type: String, value: String) -> ReferenceId {
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: None,
            kind: OutputKind::Custom { r#type, value }
        });
        ReferenceId(id)
    }

    fn create_list_raw<SEQ, T>(&mut self, name: Option<String>, item_type: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = T> + ExactSizeIterator,
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
            kind: OutputKind::List { item_type, len, value }
        });
        Ok(ReferenceId(id))
    }

    pub fn create_list<SEQ, T>(&mut self, name: String, item_type: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = T> + ExactSizeIterator,
        T: fmt::Display
    {
        self.create_list_raw(Some(name), item_type, seq)
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
            self.create_list(name, "&'static [u8]".into(), seq.map(|b| format!("&{:?}", b.as_ref())))
        }
    }

    pub fn create_limited_str_seq<SEQ>(&mut self, name: String, pool: &StringPool, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = LimitedStr> + ExactSizeIterator,
    {
        let seqid = self.create_u32_seq_raw(None, seq.map(|seq| seq.0))?;
        let id = self.list.len();
        self.list.push(OutputEntry {
            name: Some(name),
            kind: OutputKind::LimitedStrSeq { pool: pool.reference(), index: seqid }
        });
        Ok(ReferenceId(id))
    }

    pub fn create_str_seq<SEQ, B>(&mut self, name: String, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = B> + ExactSizeIterator,
        B: AsRef<str>
    {
        if seq.len() > 128 {
            let writer = self.str_writer()?;
            let offset = writer.count;
            let mut count = 0;
            let mut list = Vec::new();
            for buf in seq {
                let buf = buf.as_ref();
                writer.write_all(buf.as_bytes())?;

                let len: u32 = buf.len().try_into().unwrap();
                count += len;
                list.push(count);
            }
            let len = writer.count - offset;
            let index = self.create_u32_seq_raw(None, list.iter().copied())?;

            let id = self.list.len();
            self.list.push(OutputEntry {
                name: Some(name),
                kind: OutputKind::StrSeq { offset, len, index }
            });
            Ok(ReferenceId(id))
        } else {
            use std::fmt::Write;
            
            struct EscapeUnicode<'a>(&'a str);

            impl fmt::Display for EscapeUnicode<'_> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    for c in self.0.chars() {
                        if c.is_ascii() && !c.is_ascii_control() {
                            f.write_char(c)?;
                        } else {
                            for c in c.escape_unicode() {
                                f.write_char(c)?;
                            }
                        }
                    }

                    Ok(())
                }
            }
            
            self.create_list(
                name,
                "&'static str".into(),
                seq.map(|b| format!("\"{}\"", EscapeUnicode(b.as_ref())))
            )
        }
    }

    fn create_u32_seq_raw<SEQ>(&mut self, name: Option<String>, seq: SEQ)
        -> io::Result<ReferenceId>
    where
        SEQ: Iterator<Item = u32> + ExactSizeIterator
    {
        if seq.len() > (4 * 1024) {
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
            self.create_list_raw(name, "u32".into(), seq)
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
            r#type: String,
            value: String
        }

        let crate_name = env!("CARGO_CRATE_NAME");
        let vis = self.vis.as_deref()
            .map(|vis| format!("pub({}) ", vis))
            .unwrap_or_default();

        let bytes_count = self.bytes_writer.as_ref()
            .map(|writer| writer.count)
            .unwrap_or_default();
        if bytes_count != 0 {
            writeln!(writer,
                r#"const STATIC_{name}_BYTES: &'static [u8; {count}] = include_bytes!("{file}.bytes");"#,
                count = bytes_count,
                name = self.name.to_ascii_uppercase(),
                file = self.name
            )?;
        }

        let str_count = self.str_writer.as_ref()
            .map(|writer| writer.count)
            .unwrap_or_default();
        if str_count != 0 {
            writeln!(writer,
                r#"const STATIC_{name}_STR: &'static str = include_str!("{file}.str");"#,
                name = self.name.to_ascii_uppercase(),
                file = self.name
            )?;
        }

        let u32seq_count = self.u32seq_writer.as_ref()
            .map(|writer| writer.count)
            .unwrap_or_default();
        if u32seq_count != 0 {
            writeln!(writer,
                "\
const STATIC_{name}_U32SEQ: &[u8; {count}] = {{
    static {name}_BYTES_U32SEQ: {crate_name}::aligned::AlignedBytes<{count}, u32> =
        {crate_name}::aligned::AlignedBytes {{
            align: [],
            bytes: *include_bytes!(\"{file}.u32seq\")
        }};

    &{name}_BYTES_U32SEQ.bytes
}};\
                ",
                count = u32seq_count,
                name = self.name.to_ascii_uppercase(),
                file = self.name
            )?;
        }

        let bytes = format!("STATIC_{}_BYTES", self.name.to_ascii_uppercase());
        let str_ref = format!("STATIC_{}_STR", self.name.to_ascii_uppercase());
        let u32seq = format!("STATIC_{}_U32SEQ", self.name.to_ascii_uppercase());
        let mut list: Vec<ReferenceEntry> = Vec::with_capacity(self.list.len());

        for entry in &self.list {
            let entry = match &entry.kind {
                OutputKind::Custom { r#type, value } => ReferenceEntry {
                    r#type: r#type.clone(),
                    value: value.clone()
                },
                OutputKind::U8Seq { offset, len } => {
                    let ty = format!(
                        "{crate_name}::store::ConstSlice<'static, {}, {}, [u8; {}]>",
                        offset,
                        len,
                        bytes_count,
                    );
                    let val = format!("<{}>::new({})", ty, bytes);

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                        ReferenceEntry { r#type: ty, value: entry_name.clone() }
                    } else {
                        ReferenceEntry { r#type: ty, value: val }
                    }
                },
                OutputKind::U32Seq { offset, len } => {
                    let data_ty = format!(
                        "{crate_name}::store::ConstSlice<'static, {}, {}, [u8; {}]>",
                        offset,
                        len,
                        u32seq_count,
                    );
                    let ty = format!(
                        "{crate_name}::aligned::AlignedArray<{}, u32, {}>",
                        len,
                        data_ty,
                    );
                    let data_val = format!("<{}>::new({})", data_ty, u32seq);
                    let val = format!("<{}>::new({})", ty, data_val);

                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                        ReferenceEntry { r#type: ty, value: entry_name.clone() }
                    } else {
                        ReferenceEntry { r#type: ty, value: val }
                    }
                },
                OutputKind::BytesSeq { offset, len, index } => {
                    let ty = format!(
                        "{crate_name}::seq::CompactSeq<'static, {}, {}, {}, [u8; {}]>",
                        offset,
                        len,
                        &list[index.0].r#type,
                        bytes_count,
                    );
                    let val = format!(
                        "{crate_name}::seq::CompactSeq::new({}, {crate_name}::store::ConstSlice::new({}))",
                        &list[index.0].value,
                        bytes,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }                    
                },
                OutputKind::StrSeq { offset, len, index } => {
                    let ty = format!(
                        "{crate_name}::seq::CompactSeq<'static, {}, {}, {}, str>",
                        offset,
                        len,
                        &list[index.0].r#type,
                    );
                    let val = format!(
                        "{crate_name}::seq::CompactSeq::new({}, {crate_name}::store::ConstSlice::new({}))",
                        &list[index.0].value,
                        str_ref,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }                    
                },
                OutputKind::LimitedStrSeq { pool, index } => {
                    let ty = format!(
                        "{crate_name}::seq::LimitedSeq<{}, &'static str>",
                        &list[index.0].r#type,
                    );
                    let val = format!(
                        "{crate_name}::seq::LimitedSeq::new({}, {})",
                        &list[index.0].value,
                        pool,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }                    
                }
                OutputKind::List { item_type, value, len } => {
                    let ty = format!(
                        "{crate_name}::seq::List<'static, {}, {}>",
                        len, item_type,
                    );
                    let val = format!("{crate_name}::seq::List({})", value);
                    
                    if let Some(entry_name) = entry.name.as_ref() {
                        writeln!(writer, "{vis}const {}: {} = {};", entry_name, ty, val)?;
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
                        "{crate_name}::TinyMap<'static, {crate_name}::store::Ordered<{}>>",
                        &list[data.0].r#type,
                    );
                    let val = format!(
                        "{crate_name}::TinyMap::new({crate_name}::store::Ordered({}))",
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}static {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }
                },
                OutputKind::Small { seed, data } => {
                    let ty = format!(
                        "{crate_name}::SmallMap<'static, {}, {}>",
                        &list[data.0].r#type,
                        self.hash,
                    );
                    let val = format!(
                        "{crate_name}::SmallMap::new({}, {})",
                        seed,
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}static {}: {} = {};", entry_name, ty, val)?;
                    ReferenceEntry { r#type: ty, value: entry_name.clone() }
                },
                OutputKind::Medium { seed, slots, pilots, remap, data } => {
                    let ty = format!(
                        "{crate_name}::MediumMap<'static, {}, {}, {}, {}>",
                        &list[pilots.0].r#type,
                        &list[remap.0].r#type,
                        &list[data.0].r#type,
                        self.hash,
                    );
                    let val = format!(
                        "{crate_name}::MediumMap::new({}, {}, {}, {}, {})",
                        seed,
                        slots,
                        &list[pilots.0].value,
                        &list[remap.0].value,
                        &list[data.0].value,
                    );

                    let entry_name = entry.name.as_ref().unwrap();
                    writeln!(writer, "{vis}static {}: {} = {};", entry_name, ty, val)?;
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
