#![allow(clippy::uninlined_format_args)]

mod hash;

use std::fs;
use std::io::Write;
use precomputed_map::phf::HashOne;

fn main() {
    let mut args = std::env::args().skip(1);

    let mode = args.next();
    let num = args.next()
        .map(|num| num.parse::<u32>().unwrap())
        .unwrap_or(10000);
    let hash = args.next();

    let _ = fs::create_dir("examples");

    let map = (0u32..num)
        .map(|id| {
            let hash = hash::Default::hash_one(0, id);
            let k = format!("{:x}{}", hash, id);
            let v = (hash as u32) ^ id;
            (k, v)
        })
        .collect::<Vec<_>>();

    match mode.as_deref() {
        Some("precomputed") => precomputed(&map, hash.as_deref()),
        Some("naive") => naive(&map, hash.as_deref()),
        _ => panic!("need command")
    }
}

fn precomputed(map: &[(String, u32)], hash: Option<&str>) {
    let keys = (0..map.len()).collect::<Vec<usize>>();

    let mut map_builder = precomputed_map::builder::MapBuilder::new();
    let ord = |&x: &usize, &y: &usize| map[x].0.cmp(&map[y].0);

    let hashfn = match hash {
        Some("sip") => hash::Sip::hash_one::<&[u8]>,
        Some("xx3") => hash::Xx3::hash_one::<&[u8]>,
        Some("fx") => hash::Fx::hash_one::<&[u8]>,
        Some("fold") => hash::Fold::hash_one::<&[u8]>,
        #[cfg(feature = "gxhash")]
        Some("gx") => hash::Gx::hash_one::<&[u8]>,
        Some(_) | _ => {
            if hash.is_some() {
                eprintln!("unknown hash: {:?}", hash);
            }

            map_builder.set_ord(&ord);
            hash::Default::hash_one::<&[u8]>
        }
    };

    let hasher = match hash {
        Some("sip") => "Sip",
        Some("xx3") => "Xx3",
        Some("fx") => "Fx",
        Some("fold") => "Fold",
        Some("gx") => "Gx",
        _ => "Default"
    };
    
    let mapout = map_builder
        .set_seed(17162376839062016489)
        .set_hash(&|seed, &k|
            hashfn(seed, map[k].0.as_bytes())
        )
        .build(&keys)
        .unwrap();

    dbg!(mapout.seed());

    let mut builder = precomputed_map::builder::CodeBuilder::new(
        "str2id".into(),
        hasher.into(),
        "examples".into(),
    );

    let k = mapout.reorder(map).map(|(k, _)| k.as_bytes());
    let v = mapout.reorder(map).map(|(_, v)| *v);

    let k = builder.create_bytes_seq("STR2ID_STR".into(), k).unwrap();
    let v = builder.create_u32_seq("STR2ID_ID".into(), v).unwrap();
    let pair = builder.create_pair(k, v);

    mapout.create_map("STR2ID_MAP".into(), pair, &mut builder).unwrap();

    let mut code_file = fs::File::create("examples/str2id.rs").unwrap();
    code_file.write_all(b"#![allow(non_camel_case_types)]\n").unwrap();
    builder.write_to(&mut code_file).unwrap();

    writeln!(code_file,
        r#"
include!("../src/hash.rs");

fn main() {{
    use std::fmt::Write;
    use criterion::measurement::Measurement;

    let query = std::env::args()
        .nth(1)
        .map(|arg| arg.parse::<u32>().unwrap());

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s.as_bytes()).unwrap();
    assert_eq!(id, {});

    let timer = criterion_cycles_per_byte::CyclesPerByte;
    let mut sum = timer.zero();
    let mut buf = String::new();

    for c in 0..10 {{
        for id in 0..STR2ID_MAP.len() {{
            let id = query.unwrap_or(id as u32);
            let hash = <Default>::hash_one(0, id);
            buf.clear();
            write!(buf, "{{:x}}{{}}", hash, id).unwrap();
            let k = &buf;
            let s = std::hint::black_box(k.as_bytes());

            let start = timer.start();
            let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
            let end = timer.end(start);
            sum = timer.add(&sum, &end);
            std::hint::black_box(id);
        }}

        std::hint::black_box(c);
    }}

    println!("{{:?}}", timer.to_f64(&sum) / (STR2ID_MAP.len() * 10) as f64);
}}
        "#,
        map[1].0,
        map[1].1
    ).unwrap();
}

fn naive(map: &[(String, u32)], hash: Option<&str>) {
    let mut code_file = fs::File::create("examples/str2id_naive.rs").unwrap();

    let hasher = match hash {
        Some("sip") => "siphasher::sip::SipHasher13",
        Some("xx3") => "xxhash_rust::xxh3::Xxh3",
        Some("fx") => "rustc_hash::FxHasher",
        Some("fold") => "foldhash::fast::FoldHasher",
        Some("gx") => "gxhash::GxHasher",
        _ => "std::collections::hash_map::DefaultHasher"
    };    

    writeln!(code_file,
        r#"
use std::hash::BuildHasherDefault;
        
fn main() {{
    use std::fmt::Write;
    use std::collections::hash_map::DefaultHasher;
    use criterion::measurement::Measurement;
    use precomputed_map::phf::{{ HashOne, U64Hasher }};

    let query = std::env::args()
        .nth(1)
        .map(|arg| arg.parse::<u32>().unwrap());    

    let now = std::time::Instant::now();
    std::sync::LazyLock::force(std::hint::black_box(&STR2ID_MAP));
    println!("startup: {{:?}}", now.elapsed());

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
    assert_eq!(*id, {});

    let timer = criterion_cycles_per_byte::CyclesPerByte;
    let mut sum = timer.zero();
    let mut buf = String::new();

    for c in 0..10 {{
        for id in 0..STR2ID_MAP.len() {{
            let id = query.unwrap_or(id as u32);
            let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id as u32);
            buf.clear();
            write!(buf, "{{:x}}{{}}", hash, id).unwrap();
            let k = &buf;
            let s = std::hint::black_box(k.as_str());

            let start = timer.start();
            let id = map_get(std::hint::black_box(&STR2ID_MAP), s);
            let end = timer.end(start);
            sum = timer.add(&sum, &end);
            std::hint::black_box(id);
        }}

        std::hint::black_box(c);
    }}

    println!("{{:?}}", timer.to_f64(&sum) / (STR2ID_MAP.len() * 10) as f64);
}}

use std::collections::HashMap;

#[inline(never)]
fn map_get(map: &HashMap<&'static str, u32, BuildHasherDefault<{hasher}>>, s: &str) -> Option<u32> {{
    map.get(s).copied()
}}

static STR2ID_DATA: &'static [(&'static str, u32)] = &[
        "#,
        map[1].0,
        map[1].1
    ).unwrap();

    for (k, v) in map {
        writeln!(code_file, "(\"{}\", {}),", k, v).unwrap();
    }

    writeln!(code_file,
        r#"
];

static STR2ID_MAP: std::sync::LazyLock<HashMap<&'static str, u32, BuildHasherDefault<{hasher}>>> = std::sync::LazyLock::new(||
    STR2ID_DATA    
        .into_iter()
        .copied()
        .collect()
);
        "#
    ).unwrap();
}
