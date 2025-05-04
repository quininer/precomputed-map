use std::hash::Hash;


#[derive(Default)]
pub struct MapBuilder<K, V> {
    datas: Vec<(K, V)>,
}

impl<K: Hash + Eq, V> Extend<(K, V)> for MapBuilder<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.datas.extend(iter);
    }
}

impl<K: Hash + Eq, V> MapBuilder<K, V> {
    pub fn insert(&mut self, k: K, v: V) {
        self.datas.push((k, v));
    }
}

impl<K, V> MapBuilder<K, V> {
    pub fn build_medium(&self) {
        todo!()
    }
}
