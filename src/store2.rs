use core::marker::PhantomData;

pub trait AsData {
    type Data: ?Sized;
    
    fn as_data() -> &'static Self::Data;
}

pub trait AccessSeq {
    type Item;
    const LEN: usize;

    fn index(index: usize) -> Self::Item;
}

pub trait MapStore {
    type Key;
    type Value;

    const LEN: usize;

    fn get_key(index: usize) -> Self::Key;
    fn get_value(index: usize) -> Self::Value;    
}

pub struct SliceData<
    const O: usize,
    const L: usize,
    D
>(PhantomData<([u8; O], [u8; L], D)>);

impl<
    const B: usize,
    const O: usize,
    const L: usize,
    D: AsData<Data = [u8; B]>
> SliceData<O, L, D> {
    pub const fn new() -> Self {
        SliceData(PhantomData)
    }
}

impl<
    const B: usize,
    const O: usize,
    const L: usize,
    D: AsData<Data = [u8; B]>
> AsData for SliceData<O, L, D> {
    type Data = [u8; L];

    fn as_data() -> &'static Self::Data {
        D::as_data()[O..][..L].try_into().unwrap()
    }
}

impl<const B: usize, D> AccessSeq for D
where
    D: AsData<Data = [u8; B]>
{
    type Item = u8;
    const LEN: usize = B;

    fn index(index: usize) -> Self::Item {
        D::as_data()[index]
    }
}

impl<K> MapStore for K
where
    K: AccessSeq
{
    type Key = K::Item;
    type Value = usize;

    const LEN: usize = K::LEN;

    fn get_key(index: usize) -> Self::Key {
        K::index(index)
    }

    fn get_value(index: usize) -> Self::Value {
        index
    }
}

impl<K, V> MapStore for (K, V)
where
    K: AccessSeq,
    V: AccessSeq
{
    type Key = K::Item;
    type Value = V::Item;

    const LEN: usize = {
        if K::LEN != V::LEN {
            panic!();
        }

        K::LEN
    };

    #[inline]
    fn get_key(index: usize) -> Self::Key {
        K::index(index)
    }

    #[inline]
    fn get_value(index: usize) -> Self::Value {
        V::index(index)
    }
}
