# Precomputed Map

Precomputed a Map to embed in binary.

## What and Why

The tool attempts to achieve the best in terms of
runtime performance (include startup time and single query time), product size, and compilation speed.
and avoid any heavyweight dependency libraries and unsafe code,
make it possible to be used without supply chain burden.

Under the hood, we use binary search or phf for different sizes of data.
For small data, the runtime performance should be faster than a typical hashmap, and there is almost no size overhead.
For large data, it only requires one additional memory access at runtime, and only 1 byte of size overhead for every 3 entries.

At same time, in order to ensure the compilation speed,
the tool supports packaging a large number of strings into a single blob.
this will greatly improve the compilation performance and product size.

When there are 200,000 entries, the demo has the following comparison data:

|                | startup | single query | compile | size (stripped)
|----------------|---------|--------------|---------|----------------
|naive           | 6.6ms   | 130ns        | 6.9s    | 14M
|precomputed     | _       | 156ns        | 0.3s    | 6.3M

* Data from my Arch Linux (i7-13700H)

## Usage

For small amounts of data, it is suitable to be executed in `build.rs`.
For large amounts of data, it is better to have a separate xtask stage
to generate code without affecting development performance.

```rust,ignore
let keys: &[str] = ...;
let values: &[str] = ...;

// compute map
let mapout = precomputed_map::builder::MapBuilder::new(&keys)
    .set_seed(prev_seed)
    .set_ord(&|x, y| x.cmp(y))
    .set_hash(&|seed, &k| {
        let mut hasher = MyHasher::with_key(seed);
        k.hash(&mut hasher);
        k.finish()
    })
    .build()
    .unwrap();

// generate code
let mut builder = precomputed_map::builder::CodeBuilder::new(
    "mymap".into(),
    "MyHasher".into(),
    "src/generated".into(),
);

let k = builder.create_str_seq("MYMAP_KEYS".into(), mapout.reorder(keys)).unwrap();
let v = builder.create_str_seq("MYMAP_VALUES".into(), mapout.reorder(values)).unwrap();
let pair = builder.create_pair(k, v);

mapout.create_map("MYMAP".into(), pair, &mut builder).unwrap();

let mut codeout = fs::File::create("examples/mymap.rs").unwrap();
builder.write_to(&mut codeout).unwrap();
```

# License

This project is licensed under the MIT license.
