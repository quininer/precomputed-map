use std::cmp::Ordering;


#[derive(Default)]
pub struct MapBuilder<'f, T> {
    datas: Vec<T>,
    eq: Option<&'f dyn Fn(&T, &T) -> bool>,
    ord: Option<&'f dyn Fn(&T, &T) -> Ordering>,
    hash: Option<&'f dyn Fn(&T, u64) -> u64>,
    hash128: Option<&'f dyn Fn(&T, u64) -> u128>,
    as_bytes: Option<&'f dyn Fn(&T) -> &[u8]>,
}

impl<'f, T> MapBuilder<'f, T> {
    pub fn insert(&mut self, v: T) {
        self.datas.push(v);
    }
}

impl<'f, T> MapBuilder<'f, T> {
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
