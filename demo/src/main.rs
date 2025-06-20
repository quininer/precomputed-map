#![allow(clippy::uninlined_format_args)]

use std::fs;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use precomputed_map::phf::{ HashOne, U64Hasher };

fn main() {
    let mut args = std::env::args().skip(1);

    let mode = args.next();
    let num = args.next()
        .map(|num| num.parse::<u32>().unwrap())
        .unwrap_or(10000);
    let hash_only = matches!(args.next().as_deref(), Some("hash-only"));

    let _ = fs::create_dir("examples");

    let map = (0u32..num)
        .map(|id| {
            let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id);
            let k = format!("{:x}{}", hash, id);
            let v = (hash as u32) ^ id;
            (k, v)
        })
        .collect::<Vec<_>>();

    match mode.as_deref() {
        Some("precomputed") => precomputed(&map, hash_only),
        Some("naive") => naive(&map),
        _ => panic!("need command")
    }
}

fn precomputed(map: &[(String, u32)], hash_only: bool) {
    let keys = (0..map.len()).collect::<Vec<usize>>();

    let mut map_builder = precomputed_map::builder::MapBuilder::new(&keys);
    let ord = |&x: &usize, &y: &usize| map[x].0.cmp(&map[y].0);

    if !hash_only {
        map_builder.set_ord(&ord);
    }
    
    let mapout = map_builder
        .set_seed(17162376839062016489)
        .set_hash(&|seed, &k|
            <U64Hasher<DefaultHasher>>::hash_one(seed, map[k].0.as_str())
        )
        .build()
        .unwrap();

    dbg!(mapout.seed());

    let mut builder = precomputed_map::builder::CodeBuilder::new(
        "str2id".into(),
        "precomputed_map::phf::U64Hasher<std::collections::hash_map::DefaultHasher>".into(),
        "examples".into(),
    );

    let k = mapout.reorder(map).map(|(k, _)| k.as_str());
    let v = mapout.reorder(map).map(|(_, v)| *v);

    let k = builder.create_str_seq("STR2ID_STR".into(), k).unwrap();
    let v = builder.create_u32_seq("STR2ID_ID".into(), v).unwrap();
    let pair = builder.create_pair(k, v);

    mapout.create_map("STR2ID_MAP".into(), pair, &mut builder).unwrap();

    let mut code_file = fs::File::create("examples/str2id.rs").unwrap();
    builder.write_to(&mut code_file).unwrap();

    writeln!(code_file,
        r#"
fn main() {{
    use std::collections::hash_map::DefaultHasher;
    use precomputed_map::phf::{{ HashOne, U64Hasher }};

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
    assert_eq!(id, {});

    let mut sum = std::time::Duration::new(0, 0);

    for id in 0..STR2ID_MAP.len() {{
        let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id as u32);
        let k = format!("{{:x}}{{}}", hash, id);
        let s = std::hint::black_box(k.as_str());

        let now = std::time::Instant::now();
        let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
        sum += now.elapsed();
        std::hint::black_box(id);
    }}

    println!("{{:?}}", sum / STR2ID_MAP.len() as u32);
}}
        "#,
        map[1].0,
        map[1].1
    ).unwrap();
}

fn naive(map: &[(String, u32)]) {
    let mut code_file = fs::File::create("examples/str2id_naive.rs").unwrap();

    writeln!(code_file,
        r#"
fn main() {{
    use std::collections::hash_map::DefaultHasher;
    use precomputed_map::phf::{{ HashOne, U64Hasher }};

    let now = std::time::Instant::now();
    std::sync::LazyLock::force(std::hint::black_box(&STR2ID_MAP));
    println!("startup: {{:?}}", now.elapsed());

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
    assert_eq!(*id, {});

    let mut sum = std::time::Duration::new(0, 0);

    for id in 0..STR2ID_MAP.len() {{
        let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id as u32);
        let k = format!("{{:x}}{{}}", hash, id);
        let s = std::hint::black_box(k.as_str());

        let now = std::time::Instant::now();
        let id = map_get(std::hint::black_box(&STR2ID_MAP), s);
        sum += now.elapsed();
        std::hint::black_box(id);
    }}

    println!("{{:?}}", sum / STR2ID_MAP.len() as u32);
}}

use std::collections::HashMap;

#[inline(never)]
fn map_get(map: &HashMap<&'static str, u32>, s: &str) -> Option<u32> {{
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

static STR2ID_MAP: std::sync::LazyLock<HashMap<&'static str, u32>> = std::sync::LazyLock::new(||
    STR2ID_DATA    
        .into_iter()
        .copied()
        .collect()
);
        "#
    ).unwrap();
}
