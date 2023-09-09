// Standard Uses
use std::hash::Hash;
use std::slice::SliceIndex;

// Crate Uses

// External Uses
use bimap::BiMap;


/// TODO: This is a very explosive implementation of this that is only intended
///       to be used immutably, it would be sort-of UB if not well handled mutably
pub struct DualKeyMap<K1, K2, V> {
    pub key_map: BiMap<K1, Option<K2>>,
    pub values: Vec<V>
}

impl<K1, K2, V> DualKeyMap<K1, K2, V>
    where K1: Default + Eq + Hash + Clone + SliceIndex<[V]>,
          K2: Default + Eq + Hash,
          V: Default
{
    pub fn new() -> Self {
        Self { key_map: BiMap::<K1, Option<K2>>::new(), values: vec![] }
    }

    pub fn get_by_left(&self, left: K1) -> Option<&<K1 as SliceIndex<[V]>>::Output> {
        return Some(&self.values[left])
    }

    pub fn get_by_right(&self, right: K2) -> Option<&<K1 as SliceIndex<[V]>>::Output> {
        if let Some(left) = self.key_map.get_by_right(&Some(right)) {
            return Some(&self.values[left.clone()])
        }

        None
    }

    pub fn push_with_left(&mut self, left: K1, value: V) {
        self.key_map.insert(left, None);
        self.values.push(value);
    }

    pub fn push_with_keys(&mut self, left: K1, right: K2, value: V) {
        self.key_map.insert(left, Some(right));
        self.values.push(value);
    }
}



mod tests {
    #[allow(unused)]
    use super::DualKeyMap;

    #[test]
    fn make_map() {
        let mut map = DualKeyMap::<usize, String, String>::new();
        map.push_with_keys(0, "zero".to_owned(), "Hello".to_string());
        map.push_with_left(1,"World".to_string());


        assert_eq!(Some(&"Hello".to_owned()), map.get_by_right("zero".to_owned()));
        assert_eq!(Some(&"World".to_owned()), map.get_by_left(1));
    }
}

