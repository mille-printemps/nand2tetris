#[cfg(feature = "threadsafe")]
pub(crate) type Ref<T> = std::sync::Arc<T>;

#[cfg(not(feature = "threadsafe"))]
pub(crate) type Ref<T> = std::rc::Rc<T>;

pub mod catdeque;
pub mod deque;
pub mod hashmap;
pub mod list;
pub mod trie;
