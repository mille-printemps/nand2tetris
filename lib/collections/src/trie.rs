use crate::list::List;
use crate::Ref;

type Nodes<T, U> = List<(T, Ref<Trie<T, U>>)>;

pub struct Trie<T = u8, U = bool> {
    pub(crate) value: List<U>,
    pub(crate) nodes: Nodes<T, U>,
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
            value: List::new(),
            nodes: List::new(),
        }
    }

    pub fn empty() -> Trie<T, U> {
        Self::new()
    }

    pub fn insert_store<K: AsRef<[T]>>(&self, key: K, store: U) -> Self {
        let key_ref = key.as_ref();
        if key_ref.is_empty() {
            return Trie {
                value: self.value.push_front(store),
                nodes: self.nodes.clone(),
            };
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        Trie {
            value: self.value.clone(),
            nodes: insert_into_nodes(&self.nodes, head, tail, store),
        }
    }

    pub fn get_store<K: AsRef<[T]>>(&self, key: K) -> Option<Box<[&U]>> {
        let key_ref = key.as_ref();
        if key_ref.is_empty() {
            if self.value.is_empty() {
                return None;
            }
            let collected: Vec<&U> = self.value.iter_ref().collect();
            return Some(collected.into_boxed_slice());
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        for (k, v) in self.nodes.iter_ref() {
            if k == head {
                return v.get_store(tail);
            }
        }
        None
    }

    pub fn remove_store<V: AsRef<[T]>>(&self, key: V, store: &U) -> Option<Self> {
        let key_ref = key.as_ref();
        if key_ref.is_empty() {
            let new_value = filter_value(&self.value, store);
            if new_value.len() == self.value.len() {
                return None;
            }
            return Some(Trie {
                value: new_value,
                nodes: self.nodes.clone(),
            });
        }
        let head = &key_ref[0];
        let tail = &key_ref[1..];
        remove_from_nodes(&self.nodes, head, tail, store).map(|new_nodes| Trie {
            value: self.value.clone(),
            nodes: new_nodes,
        })
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

fn insert_into_nodes<T: PartialEq + Clone, U: PartialEq>(
    nodes: &Nodes<T, U>,
    head: &T,
    tail: &[T],
    store: U,
) -> Nodes<T, U> {
    match nodes.pop_front_rc() {
        None => List::new().push_front((
            head.clone(),
            Ref::new(Trie::new().insert_store(tail, store)),
        )),
        Some((entry, rest)) => {
            let (k, child) = entry.as_ref();
            if k == head {
                rest.push_front((k.clone(), Ref::new(child.insert_store(tail, store))))
            } else {
                insert_into_nodes(&rest, head, tail, store).push_front((k.clone(), child.clone()))
            }
        }
    }
}

// Returns a new list with every element equal to `store` removed.
fn filter_value<U: PartialEq>(values: &List<U>, store: &U) -> List<U> {
    match values.pop_front_rc() {
        None => List::new(),
        Some((v, rest)) => {
            let rest_filtered = filter_value(&rest, store);
            if v.as_ref() == store {
                rest_filtered
            } else {
                rest_filtered.push_front_rc(v)
            }
        }
    }
}

fn remove_from_nodes<T: PartialEq + Clone, U: PartialEq>(
    nodes: &Nodes<T, U>,
    head: &T,
    tail: &[T],
    store: &U,
) -> Option<Nodes<T, U>> {
    let (entry, rest) = nodes.pop_front_rc()?;
    let (k, child) = entry.as_ref();
    if k == head {
        let updated = child.remove_store(tail, store)?;
        Some(rest.push_front((k.clone(), Ref::new(updated))))
    } else {
        let rest_updated = remove_from_nodes(&rest, head, tail, store)?;
        Some(rest_updated.push_front((k.clone(), child.clone())))
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
