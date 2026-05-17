#[cfg(feature = "threadsafe")]
pub(crate) type Ref<T> = std::sync::Arc<T>;

#[cfg(not(feature = "threadsafe"))]
pub(crate) type Ref<T> = std::rc::Rc<T>;

#[cfg(not(feature = "threadsafe"))]
pub mod catdeque;
pub mod deque;
pub mod empty;
pub mod hashmap;
pub mod list;
pub mod trie;

pub use empty::Empty;
