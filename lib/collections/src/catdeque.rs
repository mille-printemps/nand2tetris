use crate::deque::{BankersDeque, BankersDequeIterator, Deque};
use crate::Ref;
use std::cell::RefCell;

type D<T> = BankersDeque<T>;

// lazy infrastructure (Thunk)
type Thunk<T> = Ref<RefCell<LazyState<T>>>;

enum LazyState<T> {
    Unevaluated(Box<dyn FnOnce() -> T>),
    Evaluated(T),
    Processing,
}

impl<T: Clone + 'static> LazyState<T> {
    fn new<F>(f: F) -> Thunk<T>
    where
        F: FnOnce() -> T + 'static,
    {
        Ref::new(RefCell::new(LazyState::Unevaluated(Box::new(f))))
    }

    fn force(thunk: &Thunk<T>) -> T {
        let mut borrow = thunk.borrow_mut();
        match &*borrow {
            LazyState::Evaluated(val) => val.clone(),
            LazyState::Unevaluated(_) => {
                match std::mem::replace(&mut *borrow, LazyState::Processing) {
                    LazyState::Unevaluated(f) => {
                        let val = f();
                        *borrow = LazyState::Evaluated(val.clone());
                        val
                    }
                    _ => unreachable!(),
                }
            }
            LazyState::Processing => panic!("Recursive forcing of thunk detected"),
        }
    }
}

// catenable deque data structure
#[derive(Clone)]
enum CatNode<T: Clone + 'static> {
    Shallow(D<T>),
    Deep {
        head: D<T>,
        middle: Thunk<D<CatenableDeque<T>>>,
        tail: D<T>,
    },
}

#[derive(Clone)]
pub struct CatenableDeque<T: Clone + 'static> {
    node: Ref<CatNode<T>>,
    len: usize,
}

impl<T: Clone + 'static> Default for CatenableDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> CatenableDeque<T> {
    pub fn new() -> Self {
        CatenableDeque {
            node: Ref::new(CatNode::Shallow(D::empty())),
            len: 0,
        }
    }

    fn shallow(d: D<T>) -> Self {
        let len = d.len();
        CatenableDeque {
            node: Ref::new(CatNode::Shallow(d)),
            len,
        }
    }

    fn deep(head: D<T>, middle: Thunk<D<CatenableDeque<T>>>, tail: D<T>, len: usize) -> Self {
        CatenableDeque {
            node: Ref::new(CatNode::Deep { head, middle, tail }),
            len,
        }
    }

    // append d1 (small) to the front of d2
    fn short_append_left(deque1: &D<T>, deque2: &D<T>) -> D<T> {
        if deque1.is_empty() {
            deque2.clone()
        } else {
            let val_rc = deque1.front().unwrap();
            deque2.push_front((*val_rc).clone())
        }
    }

    // append d2 (small) to the back of d1
    fn short_append_right(deque1: &D<T>, deque2: &D<T>) -> D<T> {
        if deque2.is_empty() {
            deque1.clone()
        } else {
            let val_rc = deque2.front().unwrap();
            deque1.push_back((*val_rc).clone())
        }
    }

    // core catenation logic
    pub fn append(&self, other: &Self) -> Self {
        use CatNode::*;
        let len = self.len + other.len;
        match (self.node.as_ref(), other.node.as_ref()) {
            (Shallow(deque1), Shallow(deque2)) => {
                if deque1.len() < 2 {
                    Self::shallow(Self::short_append_left(deque1, deque2))
                } else if deque2.len() < 2 {
                    Self::shallow(Self::short_append_right(deque1, deque2))
                } else {
                    Self::deep(
                        deque1.clone(),
                        LazyState::new(D::empty),
                        deque2.clone(),
                        len,
                    )
                }
            }
            (Shallow(deque), Deep { head, middle, tail }) => {
                if deque.len() < 2 {
                    Self::deep(
                        Self::short_append_left(deque, head),
                        middle.clone(),
                        tail.clone(),
                        len,
                    )
                } else {
                    let head_clone = head.clone();
                    let middle_clone = middle.clone();
                    let deque_clone = deque.clone();
                    let tail_clone = tail.clone();

                    let new_middle = LazyState::new(move || {
                        let forced_middle = LazyState::force(&middle_clone);
                        forced_middle.push_front(Self::shallow(head_clone))
                    });

                    Self::deep(deque_clone, new_middle, tail_clone, len)
                }
            }
            (Deep { head, middle, tail }, Shallow(deque)) => {
                if deque.len() < 2 {
                    Self::deep(
                        head.clone(),
                        middle.clone(),
                        Self::short_append_right(tail, deque),
                        len,
                    )
                } else {
                    let head_clone = head.clone();
                    let middle_clone = middle.clone();
                    let tail_clone = tail.clone();
                    let deque_clone = deque.clone();

                    let new_middle = LazyState::new(move || {
                        let forced_middle = LazyState::force(&middle_clone);
                        forced_middle.push_back(Self::shallow(tail_clone))
                    });

                    Self::deep(head_clone, new_middle, deque_clone, len)
                }
            }
            (
                Deep {
                    head: head1,
                    middle: middle1,
                    tail: tail1,
                },
                Deep {
                    head: head2,
                    middle: middle2,
                    tail: tail2,
                },
            ) => {
                let head1_clone = head1.clone();
                let middle1_clone = middle1.clone();
                let tail1_clone = tail1.clone();

                let head2_clone = head2.clone();
                let middle2_clone = middle2.clone();
                let tail2_clone = tail2.clone();

                let new_middle = LazyState::new(move || {
                    let forced_middle1 = LazyState::force(&middle1_clone);
                    let forced_middle2 = LazyState::force(&middle2_clone);

                    let left = forced_middle1.push_back(Self::shallow(tail1_clone));
                    let right = forced_middle2.push_front(Self::shallow(head2_clone));

                    // manual append for BankersDeque
                    let mut result = left;
                    let mut deque = right;
                    let mut elements = Vec::new();
                    while let Some((val, next)) = deque.pop_front() {
                        elements.push(val);
                        deque = next;
                    }
                    for val in elements {
                        result = result.push_back((*val).clone());
                    }
                    result
                });

                Self::deep(head1_clone, new_middle, tail2_clone, len)
            }
        }
    }
}

enum IterFrame<T: Clone + 'static> {
    // iterate raw elements (from a Shallow node, or head/tail of Deep)
    Data(BankersDequeIterator<T>),
    // iterate the middle queue.
    Node(BankersDequeIterator<CatenableDeque<T>>),
}

pub struct CatenableDequeIterator<T: Clone + 'static> {
    stack: Vec<IterFrame<T>>,
}

impl<T: Clone + 'static> Iterator for CatenableDequeIterator<T> {
    type Item = Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // peek at the current active iterator on the stack
            match self.stack.last_mut() {
                None => return None,

                Some(IterFrame::Data(iter)) => {
                    if let Some(val) = iter.next() {
                        return Some(val);
                    }
                }

                Some(IterFrame::Node(iter)) => {
                    if let Some(catdeque) = iter.next() {
                        use CatNode::*;

                        match catdeque.node.as_ref() {
                            Shallow(deque) => {
                                self.stack.push(IterFrame::Data(deque.iter()));
                            }
                            Deep { head, middle, tail } => {
                                // Since it's a stack (LIFO), we push tail, then middle, then head.

                                // 3. tail (Data)
                                self.stack.push(IterFrame::Data(tail.iter()));

                                // 2. middle (Recursive Nodes)
                                let forced_middle = LazyState::force(&middle);
                                self.stack.push(IterFrame::Node(forced_middle.iter()));

                                // 1. head (Data)
                                self.stack.push(IterFrame::Data(head.iter()));
                            }
                        }
                        continue;
                    }
                }
            }
            self.stack.pop();
        }
    }
}

impl<T: Clone + 'static> Deque<T> for CatenableDeque<T> {
    type Iter = CatenableDequeIterator<T>;

    fn empty() -> Self {
        Self::new()
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn len(&self) -> usize {
        self.len
    }

    fn push_front(&self, value: T) -> Self {
        match self.node.as_ref() {
            CatNode::Shallow(deque) => Self::shallow(deque.push_front(value)),
            CatNode::Deep { head, middle, tail } => Self::deep(
                head.push_front(value),
                middle.clone(),
                tail.clone(),
                self.len() + 1,
            ),
        }
    }

    fn push_back(&self, value: T) -> Self {
        match self.node.as_ref() {
            CatNode::Shallow(deque) => Self::shallow(deque.push_back(value)),
            CatNode::Deep { head, middle, tail } => Self::deep(
                head.clone(),
                middle.clone(),
                tail.push_back(value),
                self.len() + 1,
            ),
        }
    }

    fn front(&self) -> Option<Ref<T>> {
        match self.node.as_ref() {
            CatNode::Shallow(deque) => deque.front(),
            CatNode::Deep { head, .. } => head.front(),
        }
    }

    fn back(&self) -> Option<Ref<T>> {
        match self.node.as_ref() {
            CatNode::Shallow(deque) => deque.back(),
            CatNode::Deep { tail, .. } => tail.back(),
        }
    }

    fn pop_front(&self) -> Option<(Ref<T>, Self)> {
        if self.is_empty() {
            return None;
        }

        match self.node.as_ref() {
            CatNode::Shallow(deque) => {
                let (front, rest) = deque.pop_front()?;
                Some((front, Self::shallow(rest)))
            }
            CatNode::Deep { head, middle, tail } => {
                if head.len() > 1 {
                    let (head_front, head_rest) = head.pop_front()?;
                    Some((
                        head_front,
                        Self::deep(head_rest, middle.clone(), tail.clone(), self.len - 1),
                    ))
                } else {
                    let (head_front, _) = head.pop_front()?;
                    let forced_middle = LazyState::force(middle);
                    if forced_middle.is_empty() {
                        Some((head_front, Self::shallow(tail.clone())))
                    } else {
                        let (middle_front, middle_rest) = forced_middle.pop_front()?;
                        let new_head = match middle_front.node.as_ref() {
                            CatNode::Shallow(deque) => deque.clone(),
                            _ => panic!("Invariant Violated: M contents should be Shallow"),
                        };
                        let new_middle = LazyState::new(move || middle_rest);
                        Some((
                            head_front,
                            Self::deep(new_head, new_middle, tail.clone(), self.len - 1),
                        ))
                    }
                }
            }
        }
    }

    fn pop_back(&self) -> Option<(Ref<T>, Self)> {
        if self.is_empty() {
            return None;
        }

        match self.node.as_ref() {
            CatNode::Shallow(deque) => {
                let (back, rest) = deque.pop_back()?;
                Some((back, Self::shallow(rest)))
            }
            CatNode::Deep { head, middle, tail } => {
                if tail.len() > 1 {
                    let (tail_back, tail_rest) = tail.pop_back()?;
                    Some((
                        tail_back,
                        Self::deep(head.clone(), middle.clone(), tail_rest, self.len - 1),
                    ))
                } else {
                    let (tail_back, _) = tail.pop_back()?;
                    let forced_middle = LazyState::force(middle);

                    if forced_middle.is_empty() {
                        Some((tail_back, Self::shallow(head.clone())))
                    } else {
                        let (middle_front, middle_rest) = forced_middle.pop_back()?;
                        let new_tail = match middle_front.node.as_ref() {
                            CatNode::Shallow(deque) => deque.clone(),
                            _ => panic!("Invariant Violated"),
                        };
                        let new_middle = LazyState::new(move || middle_rest);
                        Some((
                            tail_back,
                            Self::deep(head.clone(), new_middle, new_tail, self.len - 1),
                        ))
                    }
                }
            }
        }
    }

    fn iter(&self) -> Self::Iter {
        let mut stack = Vec::new();
        use CatNode::*;

        match self.node.as_ref() {
            Shallow(deque) => {
                stack.push(IterFrame::Data(deque.iter()));
            }
            Deep { head, middle, tail } => {
                stack.push(IterFrame::Data(tail.iter()));

                let forced_middle = LazyState::force(middle);
                stack.push(IterFrame::Node(forced_middle.iter()));

                stack.push(IterFrame::Data(head.iter()));
            }
        }

        CatenableDequeIterator { stack }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deque(values: &[i32]) -> CatenableDeque<i32> {
        let mut d = CatenableDeque::empty();
        for &v in values {
            d = d.push_back(v);
        }
        d
    }

    #[test]
    fn test_empty_and_is_empty() {
        let d: CatenableDeque<i32> = CatenableDeque::empty();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert!(d.front().is_none());
        assert!(d.back().is_none());
        assert!(d.pop_front().is_none());
        assert!(d.pop_back().is_none());
    }

    #[test]
    fn test_push_front() {
        let d = CatenableDeque::empty()
            .push_front(1)
            .push_front(2)
            .push_front(3);

        // [3, 2, 1]
        assert!(!d.is_empty());
        assert_eq!(d.len(), 3);
        assert_eq!(*d.front().unwrap(), 3);
        assert_eq!(*d.back().unwrap(), 1);
    }

    #[test]
    fn test_push_back() {
        let d = CatenableDeque::empty()
            .push_back(1)
            .push_back(2)
            .push_back(3);

        // [1, 2, 3]
        assert!(!d.is_empty());
        assert_eq!(d.len(), 3);
        assert_eq!(*d.front().unwrap(), 1);
        assert_eq!(*d.back().unwrap(), 3);
    }

    #[test]
    fn test_pop_front_fifo() {
        let d = create_deque(&[1, 2, 3]);

        let (v1, d1) = d.pop_front().expect("Should not be empty");
        assert_eq!(*v1, 1);
        assert_eq!(d1.len(), 2);

        let (v2, d2) = d1.pop_front().expect("Should not be empty");
        assert_eq!(*v2, 2);
        assert_eq!(d2.len(), 1);

        let (v3, d3) = d2.pop_front().expect("Should not be empty");
        assert_eq!(*v3, 3);
        assert!(d3.is_empty());
    }

    #[test]
    fn test_pop_front_lifo() {
        let d = CatenableDeque::empty().push_front(1).push_front(2);

        let (v1, d1) = d.pop_front().unwrap();
        assert_eq!(*v1, 2);

        let (v2, _) = d1.pop_front().unwrap();
        assert_eq!(*v2, 1);
    }

    #[test]
    fn test_pop_back() {
        let d = create_deque(&[1, 2, 3]);

        // pop 3
        let (v1, d1) = d.pop_back().unwrap();
        assert_eq!(*v1, 3);
        assert_eq!(d1.len(), 2);
        assert_eq!(*d1.back().unwrap(), 2);

        // pop 2
        let (v2, d2) = d1.pop_back().unwrap();
        assert_eq!(*v2, 2);

        // pop 1
        let (v3, d3) = d2.pop_back().unwrap();
        assert_eq!(*v3, 1);
        assert!(d3.is_empty());
    }

    #[test]
    fn test_front_and_back_consistency() {
        let d = create_deque(&[10, 20, 30, 40]);

        assert_eq!(*d.front().unwrap(), 10);
        assert_eq!(*d.back().unwrap(), 40);
        assert_eq!(d.len(), 4); // front/back doesn't mutate or consume
    }

    #[test]
    fn test_persistence() {
        // operations do not affect previous versions
        let d0 = CatenableDeque::empty();
        let d1 = d0.push_back(1);
        let d2 = d1.push_back(2);

        assert!(d0.is_empty());
        assert_eq!(d1.len(), 1);
        assert_eq!(d2.len(), 2);

        let (_, d2_popped) = d2.pop_front().unwrap();
        assert_eq!(d2.len(), 2); // d2 still exists unmodified
        assert_eq!(d2_popped.len(), 1);
    }

    #[test]
    fn test_append_basic() {
        let left = create_deque(&[1, 2]);
        let right = create_deque(&[3, 4]);
        let combined = left.append(&right);

        assert_eq!(combined.len(), 4);
        assert_eq!(*combined.front().unwrap(), 1);
        assert_eq!(*combined.back().unwrap(), 4);

        // drain to verify order
        let mut vals = Vec::new();
        let mut curr = combined;
        while let Some((v, next)) = curr.pop_front() {
            vals.push(*v);
            curr = next;
        }
        assert_eq!(vals, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_append_with_empty() {
        let d = create_deque(&[1, 2, 3]);
        let empty = CatenableDeque::empty();

        let d_left = empty.append(&d);
        assert_eq!(d_left.len(), 3);
        assert_eq!(*d_left.front().unwrap(), 1);

        let d_right = d.append(&empty);
        assert_eq!(d_right.len(), 3);
        assert_eq!(*d_right.back().unwrap(), 3);
    }

    #[test]
    fn test_deep_append_recursive() {
        // append multiple large queues to trigger the 'Deep' node logic
        let d1 = create_deque(&[1, 2, 3, 4]);
        let d2 = create_deque(&[5, 6, 7, 8]);
        let huge = d1.append(&d2); // [1..8]
        assert_eq!(huge.len(), 8);

        // append another Deep node to the result
        let d3 = create_deque(&[9, 10, 11, 12]);
        let massive = huge.append(&d3); // [1..12]
        assert_eq!(massive.len(), 12);
        assert_eq!(*massive.front().unwrap(), 1);
        assert_eq!(*massive.back().unwrap(), 12);

        // drain to verify order
        let mut count = 0;
        let mut curr = massive;
        while let Some((v, next)) = curr.pop_front() {
            count += 1;
            assert_eq!(*v, count, "Mismatch at index {}", count);
            curr = next;
        }
        assert_eq!(count, 12);
    }

    #[test]
    fn test_deep_pop_back() {
        // correctly traverse a Deep structure backwards
        let d1 = create_deque(&[1, 2, 3]);
        let d2 = create_deque(&[4, 5, 6]);
        let combined = d1.append(&d2);

        let mut count = 6;
        let mut curr = combined;
        while let Some((v, next)) = curr.pop_back() {
            assert_eq!(*v, count);
            count -= 1;
            curr = next;
        }
        assert_eq!(count, 0);
    }

    #[test]
    fn test_iter() {
        let d = create_deque(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        let mut iter = d.iter();
        for i in 1..=10 {
            let expected = i * 10;
            assert_eq!(iter.next(), Some(Ref::new(expected)));
        }
        assert!(iter.next().is_none());
    }
}
