#![cfg(feature = "builder")]

use std::fs;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use static_datamap::phf::{ HashOne, U64Hasher };

fn main() {
    let mut args = std::env::args().skip(1);

    let map = (0u32..10000)
        .map(|id| {
            let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id);
            let k = format!("{:x}{}", hash, id);
            let v = (hash as u32) ^ id;
            (k, v)
        })
        .collect::<Vec<_>>();

    match args.next().as_deref() {
        Some("datamap") => datamap(&map),
        Some("naive") => naive(&map),
        _ => panic!()
    }
}


fn datamap(map: &[(String, u32)]) {
    let keys = (0..map.len()).collect::<Vec<usize>>();
    
    let mapout = static_datamap::builder::MapBuilder::new(&keys)
        .set_seed(17162376839062016489)
        .set_ord(&|&x, &y| std::cmp::Ord::cmp(&map[x].0, &map[y].0))
        .set_hash(&|seed, &k|
            <U64Hasher<DefaultHasher>>::hash_one(seed, map[k].0.as_bytes())
        )
        .build()
        .unwrap();

    dbg!(mapout.seed());

    let bytes_file = fs::File::create("examples/str2id.bytes").unwrap();
    let u32seq_file = fs::File::create("examples/str2id.u32seq").unwrap();
    let mut builder = static_datamap::builder::OutputBuilder::new(
        "str2id".into(),
        "static_datamap::phf::U64Hasher<std::collections::hash_map::DefaultHasher>".into(),
        bytes_file,
        u32seq_file
    );

    let k = mapout.reorder(&map).map(|(k, _)| k.as_str());
    let v = mapout.reorder(&map).map(|(_, v)| *v);

    let k = builder.create_bytes_seq("STR2ID_STR".into(), k).unwrap();
    let v = builder.create_u32_seq("STR2ID_ID".into(), v).unwrap();
    let pair = builder.create_pair(k, v);

    mapout.create_map("STR2ID_MAP".into(), pair, &mut builder).unwrap();

    let mut code_file = fs::File::create("examples/str2id.rs").unwrap();
    builder.build(&mut code_file).unwrap();

    writeln!(code_file,
        r#"
fn main() {{
    use std::collections::hash_map::DefaultHasher;
    use static_datamap::phf::{{ HashOne, U64Hasher }};

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s.as_bytes()).unwrap();
    assert_eq!(id, {});

    let mut sum = std::time::Duration::new(0, 0);

    for id in 0..STR2ID_STR.len() {{
        let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id as u32);
        let k = format!("{{:x}}{{}}", hash, id);
        let s = std::hint::black_box(k.as_bytes());

        let now = std::time::Instant::now();
        for _ in 0..10 {{
            let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
            std::hint::black_box(id);
        }}
        sum += now.elapsed() / 10;
    }}

    println!("{{:?}}", sum);
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
    use static_datamap::phf::{{ HashOne, U64Hasher }};

    let s = std::hint::black_box({:?});
    let id = std::hint::black_box(&STR2ID_MAP).get(s.as_bytes()).unwrap();
    assert_eq!(*id, {});

    let mut sum = std::time::Duration::new(0, 0);

    for id in 0..STR2ID_MAP.len() {{
        let hash = <U64Hasher<DefaultHasher>>::hash_one(0, id as u32);
        let k = format!("{{:x}}{{}}", hash, id);
        let s = std::hint::black_box(k.as_bytes());

        let now = std::time::Instant::now();
        for _ in 0..10 {{
            let id = std::hint::black_box(&STR2ID_MAP).get(s).unwrap();
            std::hint::black_box(id);
        }}
        sum += now.elapsed() / 10;
    }}

    println!("{{:?}}", sum);
}}

use std::collections::HashMap;

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

static STR2ID_MAP: std::sync::LazyLock<HashMap<&'static [u8], u32>> = std::sync::LazyLock::new(||
    STR2ID_DATA    
        .into_iter()
        .map(|(s, id)| (s.as_bytes(), *id))
        .collect()
);
        "#
    ).unwrap();
}
