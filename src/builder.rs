use std::hash::Hash;
use std::marker::PhantomData;


pub struct MapBuilder<K: Hash + Eq> {
    _phantom: PhantomData<K>
}

impl<K: Hash + Eq> MapBuilder<K> {
    //
}
