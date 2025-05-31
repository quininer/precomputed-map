use std::hash::Hash;
use std::marker::PhantomData;
use crate::Hasher128;


#[derive(Default)]
pub struct MapBuilder<K, V, H> {
    datas: Vec<(K, V)>,
    _phantom: PhantomData<H>
}

impl<K, V, H> Extend<(K, V)> for MapBuilder<K, V, H> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.datas.extend(iter);
    }
}

impl<K, V, H> MapBuilder<K, V, H> {
    pub fn insert(&mut self, k: K, v: V) {
        self.datas.push((k, v));
    }
}

impl<K, V, H> MapBuilder<K, V, H>
where
    K: Hash + Eq,
    H: Hasher128
{
    pub fn build_tiny(&self) {
        todo!()
    }

    pub fn build_small(&self) {
        todo!()
    }
    
    pub fn build_medium(&self) {
        todo!()
    }
}
