#[cfg(feature = "threadsafe")]
pub(crate) type Ref<A> = std::sync::Arc<A>;

#[cfg(not(feature = "threadsafe"))]
pub(crate) type Ref<A> = std::rc::Rc<A>;

pub mod deque;
pub mod hashmap;
pub mod list;
pub mod trie;
