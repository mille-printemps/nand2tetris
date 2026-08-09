use crate::Ref;
use std::cell::RefCell;

enum LazyState<T> {
    Unevaluated(Box<dyn FnOnce() -> T>),
    Evaluated(T),
    Forcing,
}

// A memoising suspension
// Amortised bounds on lazy data structures depend on the memoisation
// a suspension is evaluated at most once no matter how many versions share it.
pub struct Lazy<T> {
    state: Ref<RefCell<LazyState<T>>>,
}

impl<T> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        Lazy {
            state: self.state.clone(),
        }
    }
}

impl<T: Clone + 'static> Lazy<T> {
    pub fn new<F>(compute: F) -> Self
    where
        F: FnOnce() -> T + 'static,
    {
        Lazy {
            state: Ref::new(RefCell::new(LazyState::Unevaluated(Box::new(compute)))),
        }
    }

    // A suspension that is already evaluated.
    // Avoids boxing a closure that would only return a value we already hold.
    pub fn from_value(value: T) -> Self {
        Lazy {
            state: Ref::new(RefCell::new(LazyState::Evaluated(value))),
        }
    }

    pub fn force(&self) -> T {
        let current = std::mem::replace(&mut *self.state.borrow_mut(), LazyState::Forcing);
        match current {
            LazyState::Evaluated(value) => {
                *self.state.borrow_mut() = LazyState::Evaluated(value.clone());
                value
            }
            LazyState::Unevaluated(compute) => {
                let value = compute();
                *self.state.borrow_mut() = LazyState::Evaluated(value.clone());
                value
            }
            LazyState::Forcing => panic!("recursive forcing of a suspension"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_evaluates_at_most_once() {
        let calls = Ref::new(Cell::new(0));
        let counter = calls.clone();
        let lazy = Lazy::new(move || {
            counter.set(counter.get() + 1);
            41
        });

        assert_eq!(calls.get(), 0, "not evaluated before the first force");
        assert_eq!(lazy.force(), 41);
        assert_eq!(lazy.force(), 41);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn test_memoisation_is_shared_between_clones() {
        let calls = Ref::new(Cell::new(0));
        let counter = calls.clone();
        let lazy = Lazy::new(move || {
            counter.set(counter.get() + 1);
            7
        });
        let alias = lazy.clone();

        assert_eq!(lazy.force(), 7);
        assert_eq!(alias.force(), 7);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn test_from_value_is_already_evaluated() {
        assert_eq!(Lazy::from_value("done").force(), "done");
    }
}
