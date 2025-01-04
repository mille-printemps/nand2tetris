use crate::Ref;
pub struct Trie<T = u8, U = bool> {
    pub(crate) value: Vec<Ref<U>>,
    pub(crate) nodes: Vec<(T, Ref<Trie<T, U>>)>,
}

impl<T: Clone, U> Clone for Trie<T, U> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            nodes: self.nodes.clone(),
        }
    }
}

impl<T: PartialEq + Clone, U: PartialEq> Trie<T, U> {
    pub fn new() -> Trie<T, U> {
        Trie {
            value: Vec::new(),
            nodes: Vec::new(),
        }
    }

    pub fn insert_store<K: AsRef<[T]>>(&self, key: K, store: U) -> Self {
        let key_ref = key.as_ref();
        let mut new_trie = self.clone();
        if key_ref.is_empty() {
            new_trie.value.push(Ref::new(store));
            return new_trie;
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        for (k, v) in new_trie.nodes.iter_mut() {
            if k == head {
                *v = Ref::new(v.insert_store(tail, store));
                return new_trie;
            }
        }
        new_trie.nodes.push((
            head.clone(),
            Ref::new(Trie::new().insert_store(tail, store)),
        ));
        new_trie
    }

    pub fn get_store<K: AsRef<[T]>>(&self, key: K) -> Option<Box<[&U]>> {
        let key_ref = key.as_ref();
        if key_ref.is_empty() {
            let mut vr = Vec::new();
            for v in self.value.iter() {
                vr.push(v.as_ref());
            }
            if vr.is_empty() {
                return Option::None;
            }
            return Option::Some(vr.into_boxed_slice());
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        for (k, v) in &self.nodes {
            if k == head {
                return v.get_store(tail);
            }
        }
        Option::None
    }

    pub fn remove_store<V: AsRef<[T]>>(&self, key: V, store: &U) -> Option<Self> {
        let key_ref = key.as_ref();
        let mut new_trie = self.clone();
        if key_ref.is_empty() {
            new_trie.value.retain(|v| {
                let retain = v.as_ref() != store;
                retain
            });
            if self.value.len() == new_trie.value.len() {
                return Option::None;
            } else {
                return Option::Some(new_trie);
            }
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        for (k, v) in new_trie.nodes.iter_mut() {
            if k == head {
                let subt = v.remove_store(tail, store)?;
                *v = Ref::new(subt);
                return Option::Some(new_trie);
            }
        }
        Option::None
    }
}

impl<T: PartialEq + Clone, U: PartialEq> Default for Trie<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq + Copy> Trie<T> {
    pub fn insert<V: AsRef<[T]>>(&self, value: V) -> Self {
        self.insert_store(value, true)
    }
    pub fn search<V: AsRef<[T]>>(&self, value: V) -> bool {
        self.get_store(value).is_some()
    }
    pub fn remove<V: AsRef<[T]>>(&self, value: V) -> Option<Self> {
        self.remove_store(value, &true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_trie_store() {
        let t = Trie::new().insert_store("aab", 123);
        let t2 = t.insert_store("adc", 459);
        let boxed_array: Box<[&i32]> = Box::new([&123]);
        let boxed_array_2: Box<[&i32]> = Box::new([&459]);
        assert_eq!(t.get_store("aab"), Option::Some(boxed_array.clone()));
        assert!(t.get_store("adc").is_none());
        assert_eq!(t2.get_store("aab"), Option::Some(boxed_array));
        assert_eq!(t2.get_store("adc"), Option::Some(boxed_array_2));
    }

    #[test]
    fn test_trie_persistance_simple() {
        let t = Trie::new().insert("aab").insert("adc");
        assert!(t.search("aab"));
        assert!(t.search("adc"));
    }

    #[test]
    fn test_trie_persistance() {
        let vs = vec!["aab", "adc", "acd", "dca"];
        let snapshots: Vec<_> = vs
            .iter()
            .scan(Trie::new(), |tree, value| {
                *tree = tree.insert(value);
                Option::Some(tree.clone())
            })
            .collect();
        for (index, tree) in snapshots.iter().enumerate() {
            let found = vs
                .iter()
                .map(|s| tree.search(s))
                .filter(|found| *found == true)
                .count();
            assert_eq!(found, index + 1);
        }
    }

    #[test]
    fn test_search_present() {
        let v = vec![1, 5, 9];
        let not_v = vec![1, 15, 9];
        let t = Trie::new().insert(&v);
        assert!(t.search(v));
        assert!(!t.search(not_v));
    }

    #[test]
    fn test_search_absent() {
        let s = "test";
        let not_s = "tett";
        let t = Trie::new().insert(s);
        assert!(t.search(s));
        assert!(!t.search(not_s));
    }

    #[test]
    fn test_trie_deletion() {
        let t = Trie::new().insert("aab").remove("aab");
        assert!(t.is_some());
        assert_eq!(t.unwrap().search("aab"), false);
        let t2 = Trie::new();
        assert!(t2.remove("a").is_none());
    }

    #[test]
    fn test_insert_empty_string() {
        let t = Trie::new().insert("");
        assert!(t.search(""));
    }

    #[test]
    fn test_multiple_values_for_same_key() {
        let t = Trie::new().insert_store("key", 1).insert_store("key", 2);
        let values = t.get_store("key").unwrap();
        assert!(values.contains(&&1) && values.contains(&&2));
    }

    #[test]
    fn test_delete_internal_node() {
        let t = Trie::new().insert("abc").insert("ab").remove("ab").unwrap();
        assert!(!t.search("ab"));
        assert!(t.search("abc"));
    }

    #[test]
    fn test_persistence_after_delete() {
        let t1 = Trie::new().insert("key");
        let t2 = t1.remove("key").unwrap_or_else(|| t1.clone());
        assert!(t1.search("key"));
        assert!(!t2.search("key"));
    }

    #[test]
    fn test_search_nonexistent_key() {
        let t = Trie::new().insert("key");
        assert!(!t.search("not_key"));
    }

    #[test]
    fn test_delete_nonexistent_key() {
        let t = Trie::new().insert("key");
        assert!(t.remove("not_key").is_none());
    }
}
