use std::fmt;

use crate::Ref;

use super::list;

pub trait Deque<T>: Sized + Clone {
    type Iter: Iterator<Item = Ref<T>>;

    fn empty() -> Self;
    fn push_front(&self, value: T) -> Self;
    fn push_back(&self, value: T) -> Self;
    fn pop_front(&self) -> Option<(Ref<T>, Self)>;
    fn pop_back(&self) -> Option<(Ref<T>, Self)>;
    fn front(&self) -> Option<Ref<T>>;
    fn back(&self) -> Option<Ref<T>>;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn iter(&self) -> Self::Iter;
}

pub struct BankersDeque<T> {
    head: list::List<T>,
    tail: list::List<T>,
}

impl<T> Default for BankersDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for BankersDeque<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Clone for BankersDeque<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
            tail: self.tail.clone(),
        }
    }
}

impl<T> BankersDeque<T> {
    pub fn new() -> Self {
        Self {
            head: list::List::new(),
            tail: list::List::new(),
        }
    }
}

impl<T: Clone> Deque<T> for BankersDeque<T> {
    type Iter = BankersDequeIterator<T>;

    fn empty() -> Self {
        Self::new()
    }

    fn push_front(&self, value: T) -> Self {
        Self {
            head: self.head.push_front(value),
            tail: self.tail.clone(),
        }
        .balance()
    }

    fn push_back(&self, value: T) -> Self {
        Self {
            head: self.head.clone(),
            tail: self.tail.push_front(value),
        }
        .balance()
    }

    fn pop_front(&self) -> Option<(Ref<T>, Self)> {
        if self.is_empty() {
            None
        } else if self.head.is_empty() {
            let (a, b) = self.tail.pop_front_rc()?;
            Some((
                a,
                Self {
                    head: self.head.clone(),
                    tail: b,
                }
                .balance(),
            ))
        } else {
            let (a, b) = self.head.pop_front_rc()?;
            Some((
                a,
                Self {
                    head: b,
                    tail: self.tail.clone(),
                }
                .balance(),
            ))
        }
    }

    fn pop_back(&self) -> Option<(Ref<T>, Self)> {
        if self.is_empty() {
            None
        } else if self.tail.is_empty() {
            let (a, b) = self.head.pop_front_rc()?;
            Some((
                a,
                Self {
                    head: b,
                    tail: self.tail.clone(),
                }
                .balance(),
            ))
        } else {
            let (a, b) = self.tail.pop_front_rc()?;
            Some((
                a,
                Self {
                    head: self.head.clone(),
                    tail: b,
                }
                .balance(),
            ))
        }
    }

    fn front(&self) -> Option<Ref<T>> {
        if self.is_empty() {
            None
        } else if self.head.is_empty() {
            self.tail.front_rc()
        } else {
            self.head.front_rc()
        }
    }

    fn back(&self) -> Option<Ref<T>> {
        if self.is_empty() {
            None
        } else if self.tail.is_empty() {
            self.head.front_rc()
        } else {
            self.tail.front_rc()
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }

    fn iter(&self) -> Self::Iter {
        BankersDequeIterator {
            head_iter: self.head.iter(),
            tail_iter: self.tail.reverse().iter(),
        }
    }
}

impl<T> BankersDeque<T> {
    fn balance(&self) -> Self {
        if self.head.is_empty() {
            let (tail, rev_head) = self.tail.split();
            let head = rev_head.reverse();
            Self { head, tail }
        } else if self.tail.is_empty() {
            let (head, rev_tail) = self.head.split();
            let tail = rev_tail.reverse();
            Self { head, tail }
        } else {
            self.clone()
        }
    }
}

pub struct BankersDequeIterator<T> {
    head_iter: list::ListIterator<T>,
    tail_iter: list::ListIterator<T>,
}

impl<T> Iterator for BankersDequeIterator<T> {
    type Item = Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.head_iter.next().or_else(|| self.tail_iter.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let deque: BankersDeque<i32> = BankersDeque::new();
        let deque = deque.push_front(1).push_back(2).push_front(0).push_back(3);
        assert_eq!(deque.len(), 4);

        let (value, deque) = deque.pop_front().unwrap();
        assert_eq!(*value, 0);
        let (value, deque) = deque.pop_back().unwrap();
        assert_eq!(*value, 3);
        let (value, deque) = deque.pop_front().unwrap();
        assert_eq!(*value, 1);
        let (value, deque) = deque.pop_back().unwrap();
        assert_eq!(*value, 2);

        assert_eq!(deque.len(), 0);
        assert!(deque.pop_front().is_none());
        assert!(deque.pop_back().is_none());
    }

    #[test]
    fn test_iter() {
        let deque: BankersDeque<String> = BankersDeque::new();
        let deque = deque
            .push_front("World".to_string())
            .push_front("Hello".to_string());
        let mut iter = deque.iter();
        assert_eq!(iter.next(), Some(Ref::new("Hello".to_string())));
        assert_eq!(iter.next(), Some(Ref::new("World".to_string())));
        assert_eq!(iter.next(), None);
    }
}
