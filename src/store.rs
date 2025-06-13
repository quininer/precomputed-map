use core::borrow::Borrow;
use core::marker::PhantomData;
use crate::seq::List;


pub trait AsData {
    type Data;
    
    fn as_data(&self) -> Self::Data;
}

pub trait MapStore<'data> {
    type Key: 'data;
    type Value: 'data;

    const LEN: usize;
    
    fn get_key(&self, index: usize) -> Self::Key;
    fn get_value(&self, index: usize) -> Self::Value;
}

pub trait AccessSeq<'data> {
    type Item: 'data;
    const LEN: usize;

    fn index(&self, index: usize) -> Self::Item;
}

pub trait Searchable<'data>: MapStore<'data> {
    fn search<Q>(&self, key: &Q) -> Option<Self::Value>
    where
        Self::Key: Borrow<Q>,
        Q: Ord + ?Sized
    ;
}

pub struct ConstSlice<'data, const O: usize, const L: usize, B: ?Sized> {
    data: &'data B,
    _phantom: PhantomData<([u8; O], [u8; L])>
}

pub struct Ordered<D>(pub D);

impl<'data, const O: usize, const L: usize, B: ?Sized> ConstSlice<'data, O, L, B> {
    pub const fn new(data: &'data B) -> Self {
        ConstSlice { data, _phantom: PhantomData }
    }
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const N: usize,
> AsData for ConstSlice<'data, O, N, [u8; B]> {
    type Data = &'data [u8; N];
    
    #[inline]
    fn as_data(&self) -> &'data [u8; N] {
        self.data[O..][..N].try_into().unwrap()
    }
}

impl<
    'data,
    const O: usize,
    const N: usize,
> AsData for ConstSlice<'data, O, N, str> {
    type Data = &'data str;
    
    #[inline]
    fn as_data(&self) -> &'data str {
        self.data[O..][..N].try_into().unwrap()
    }
}

impl<
    'data,
    const B: usize,
    const O: usize,
    const L: usize,
> AccessSeq<'data> for ConstSlice<'data, O, L, [u8; B]> {
    type Item = u8;
    const LEN: usize = L;

    #[inline]
    fn index(&self, index: usize) -> Self::Item {
        self.as_data()[index]
    }
}

impl<'data, K> MapStore<'data> for K
where
    K: AccessSeq<'data>
{
    type Key = K::Item;
    type Value = usize;

    const LEN: usize = K::LEN;

    #[inline]
    fn get_key(&self, index: usize) -> Self::Key {
        self.index(index)
    }

    #[inline]
    fn get_value(&self, index: usize) -> Self::Value {
        index
    }
}

impl<'data, K, V> MapStore<'data> for (K, V)
where
    K: AccessSeq<'data>,
    V: AccessSeq<'data>
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
    fn get_key(&self, index: usize) -> Self::Key {
        self.0.index(index)
    }

    #[inline]
    fn get_value(&self, index: usize) -> Self::Value {
        self.1.index(index)
    }
}

impl<'data, M> MapStore<'data> for Ordered<M>
where
    M: MapStore<'data>
{
    type Key = M::Key;
    type Value = M::Value;

    const LEN: usize = M::LEN;


    #[inline]
    fn get_key(&self, index: usize) -> Self::Key {
        self.0.get_key(index)
    }

    #[inline]
    fn get_value(&self, index: usize) -> Self::Value {
        self.0.get_value(index)
    }
}

impl<'data, const N: usize, T> Searchable<'data> for Ordered<List<'data, N, T>>
where
    T: Copy
{
    fn search<Q>(&self, key: &Q) -> Option<Self::Value>
    where
        Self::Key: core::borrow::Borrow<Q>,
        Q: Ord + ?Sized
    {
        self.0.0.binary_search_by(|t| t.borrow().cmp(key)).ok()
    }
}

impl<'data, const N: usize, K, V> Searchable<'data> for Ordered<(List<'data, N, K>, V)>
where
    K: Copy,
    V: AccessSeq<'data>
{
    fn search<Q>(&self, key: &Q) -> Option<Self::Value>
    where
        Self::Key: core::borrow::Borrow<Q>,
        Q: Ord + ?Sized
    {
        let index = self.0.0.0.binary_search_by(|t| t.borrow().cmp(key)).ok()?;
        Some(self.0.1.index(index))
    }
}
