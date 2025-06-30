#[macro_export]
macro_rules! define {
    ( const $name:ident: &[u8; $n:expr] = $path:literal ) => {
        struct $name;

        impl $crate::store2::AsData for $name {
            type Data = [u8; $n];

            fn as_data() -> &'static Self::Data {
                const $name: &[u8; $n] = include_bytes!($path);
                $name
            }
        }
    };
    ( const $name:ident: &[u32; $n:expr] = $path:literal ) => {
        struct $name;

        impl $crate::store2::AsData for $name {
            type Data = [u8; $n];

            fn as_data() -> &'static Self::Data {
                static $name: &$crate::aligned2::AlignedBytes<$n, u32> = &$crate::aligned2::AlignedBytes {
                    align: [],
                    bytes: *include_bytes!($path)
                };

                &$name.bytes
            }
        }
    };
}
