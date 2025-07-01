use core::borrow::Borrow;
use core::marker::PhantomData;

pub trait AsData {
    type Data: ?Sized;
    
    fn as_data() -> &'static Self::Data;
}

pub trait AccessSeq {
    type Item;
    const LEN: usize;

    fn index(index: usize) -> Option<Self::Item>;
}

pub trait Searchable: MapStore {
    fn search<Q>(key: &Q) -> Option<Self::Value>
    where
        Self::Key: Borrow<Q>,
        Q: Ord + ?Sized
    ;
}

pub trait MapStore {
    type Key;
    type Value;

    const LEN: usize;

    fn get_key(index: usize) -> Option<Self::Key>;
    fn get_value(index: usize) -> Option<Self::Value>;    
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
> AsData for SliceData<O, L, D> {
    type Data = [u8; L];

    #[inline(always)]
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

    #[inline(always)]
    fn index(index: usize) -> Option<Self::Item> {
        D::as_data().get(index).copied()
    }
}

impl<K> MapStore for K
where
    K: AccessSeq
{
    type Key = K::Item;
    type Value = usize;

    const LEN: usize = K::LEN;

    #[inline(always)]
    fn get_key(index: usize) -> Option<Self::Key> {
        K::index(index)
    }

    #[inline(always)]
    fn get_value(index: usize) -> Option<Self::Value> {
        Some(index)
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

    #[inline(always)]
    fn get_key(index: usize) -> Option<Self::Key> {
        K::index(index)
    }

    #[inline(always)]
    fn get_value(index: usize) -> Option<Self::Value> {
        V::index(index)
    }
}
