#![cfg(feature = "builder")]

use std::fs;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;

fn main() {
    let map = (0u32..10000)
        .map(|id| (id.to_string(), id ^ 0x42))
        .collect::<Vec<_>>();
    let keys = (0..10000).collect::<Vec<usize>>();

    let mapout = static_datamap::builder::MapBuilder::new(&keys)
        .set_seed(17162376839062016489)
        .set_ord(&|&x, &y| std::cmp::Ord::cmp(&map[x].0, &map[y].0))
        .set_hash(&|seed, &k| {
            use static_datamap::phf::{ HashOne, U64Hasher };

            <U64Hasher<DefaultHasher>>::hash_one(seed, map[k].0.as_bytes())
        })
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
    let s = "1";
    let id = STR2ID_MAP.get(s.as_bytes()).unwrap();
    assert_eq!(id, 1 ^ 0x42);
}}
        "#
    ).unwrap();
}
