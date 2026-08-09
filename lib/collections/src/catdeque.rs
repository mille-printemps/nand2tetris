// A catenable deque: a persistent deque that also supports append.

use crate::deque::{BankersDeque, BankersDequeIterator, Deque};
use crate::lazy::Lazy;
use crate::Ref;
use std::fmt;

// A run of consecutive elements. Okasaki's `D.Queue`.
type Chunk<T> = BankersDeque<Elem<T>>;

// One slot of a chunk.
enum Elem<T> {
    Value(Ref<T>),
    Chunk(Chunk<T>),
}

impl<T> Clone for Elem<T> {
    fn clone(&self) -> Self {
        match self {
            Elem::Value(value) => Elem::Value(value.clone()),
            Elem::Chunk(chunk) => Elem::Chunk(chunk.clone()),
        }
    }
}

enum CatNode<T> {
    Shallow(Chunk<T>),
    // Invariant: head and tail are both non-empty.
    Deep {
        head: Chunk<T>,
        middle: Lazy<Spine<T>>,
        tail: Chunk<T>,
    },
}

// A catenable deque of Elem: the recursive structure itself.
struct Spine<T> {
    node: Ref<CatNode<T>>,
}

impl<T> Clone for Spine<T> {
    fn clone(&self) -> Self {
        Spine {
            node: self.node.clone(),
        }
    }
}

impl<T: 'static> Spine<T> {
    fn shallow(chunk: Chunk<T>) -> Self {
        Spine {
            node: Ref::new(CatNode::Shallow(chunk)),
        }
    }

    fn deep(head: Chunk<T>, middle: Lazy<Spine<T>>, tail: Chunk<T>) -> Self {
        debug_assert!(!head.is_empty(), "Deep head must be non-empty");
        debug_assert!(!tail.is_empty(), "Deep tail must be non-empty");
        Spine {
            node: Ref::new(CatNode::Deep { head, middle, tail }),
        }
    }

    fn empty() -> Self {
        Self::shallow(BankersDeque::new())
    }

    fn cons(&self, elem: Elem<T>) -> Self {
        match self.node.as_ref() {
            CatNode::Shallow(chunk) => Self::shallow(chunk.push_front(elem)),
            CatNode::Deep { head, middle, tail } => {
                Self::deep(head.push_front(elem), middle.clone(), tail.clone())
            }
        }
    }

    fn snoc(&self, elem: Elem<T>) -> Self {
        match self.node.as_ref() {
            CatNode::Shallow(chunk) => Self::shallow(chunk.push_back(elem)),
            CatNode::Deep { head, middle, tail } => {
                Self::deep(head.clone(), middle.clone(), tail.push_back(elem))
            }
        }
    }

    fn head(&self) -> Option<Elem<T>> {
        let chunk = match self.node.as_ref() {
            CatNode::Shallow(chunk) => chunk,
            CatNode::Deep { head, .. } => head,
        };
        chunk.front().map(|elem| (*elem).clone())
    }

    fn last(&self) -> Option<Elem<T>> {
        let chunk = match self.node.as_ref() {
            CatNode::Shallow(chunk) => chunk,
            CatNode::Deep { tail, .. } => tail,
        };
        chunk.back().map(|elem| (*elem).clone())
    }

    fn append(&self, other: &Self) -> Self {
        match (self.node.as_ref(), other.node.as_ref()) {
            (CatNode::Shallow(left), CatNode::Shallow(right)) => {
                if left.len() < 2 {
                    Self::shallow(left.append(right))
                } else if right.len() < 2 {
                    Self::shallow(left.push_back_all(right))
                } else {
                    Self::deep(left.clone(), Lazy::from_value(Self::empty()), right.clone())
                }
            }

            (CatNode::Shallow(chunk), CatNode::Deep { head, middle, tail }) => {
                if chunk.len() < 2 {
                    Self::deep(chunk.append(head), middle.clone(), tail.clone())
                } else {
                    let (spine, displaced) = (middle.clone(), head.clone());
                    Self::deep(
                        chunk.clone(),
                        Lazy::new(move || spine.force().cons(Elem::Chunk(displaced))),
                        tail.clone(),
                    )
                }
            }

            (CatNode::Deep { head, middle, tail }, CatNode::Shallow(chunk)) => {
                if chunk.len() < 2 {
                    Self::deep(head.clone(), middle.clone(), tail.push_back_all(chunk))
                } else {
                    let (spine, displaced) = (middle.clone(), tail.clone());
                    Self::deep(
                        head.clone(),
                        Lazy::new(move || spine.force().snoc(Elem::Chunk(displaced))),
                        chunk.clone(),
                    )
                }
            }

            (
                CatNode::Deep {
                    head: left_head,
                    middle: left_middle,
                    tail: left_tail,
                },
                CatNode::Deep {
                    head: right_head,
                    middle: right_middle,
                    tail: right_tail,
                },
            ) => {
                let (left_spine, right_spine) = (left_middle.clone(), right_middle.clone());
                let (displaced_left, displaced_right) = (left_tail.clone(), right_head.clone());
                // The recursive step: the two spines are catenated by *this* function one level down,
                // not merged element by element.
                let middle = Lazy::new(move || {
                    let left = left_spine.force().snoc(Elem::Chunk(displaced_left));
                    let right = right_spine.force().cons(Elem::Chunk(displaced_right));
                    left.append(&right)
                });
                Self::deep(left_head.clone(), middle, right_tail.clone())
            }
        }
    }

    fn tail(&self) -> Option<Self> {
        match self.node.as_ref() {
            CatNode::Shallow(chunk) => Some(Self::shallow(chunk.pop_front()?.1)),
            CatNode::Deep { head, middle, tail } => {
                let (_, rest) = head.pop_front()?;
                if !rest.is_empty() {
                    return Some(Self::deep(rest, middle.clone(), tail.clone()));
                }
                // head is used up. refill it from the front of the spine.
                let spine = middle.force();
                match spine.head() {
                    None => Some(Self::shallow(tail.clone())),
                    Some(Elem::Chunk(chunk)) => Some(Self::deep(
                        chunk,
                        Lazy::from_value(spine.tail().expect("a non-empty spine has a tail")),
                        tail.clone(),
                    )),
                    Some(Elem::Value(_)) => unreachable!("a spine holds only chunks"),
                }
            }
        }
    }

    fn init(&self) -> Option<Self> {
        match self.node.as_ref() {
            CatNode::Shallow(chunk) => Some(Self::shallow(chunk.pop_back()?.1)),
            CatNode::Deep { head, middle, tail } => {
                let (_, rest) = tail.pop_back()?;
                if !rest.is_empty() {
                    return Some(Self::deep(head.clone(), middle.clone(), rest));
                }
                let spine = middle.force();
                match spine.last() {
                    None => Some(Self::shallow(head.clone())),
                    Some(Elem::Chunk(chunk)) => Some(Self::deep(
                        head.clone(),
                        Lazy::from_value(spine.init().expect("a non-empty spine has an init")),
                        chunk,
                    )),
                    Some(Elem::Value(_)) => unreachable!("a spine holds only chunks"),
                }
            }
        }
    }
}

// public wrapper

pub struct CatenableDeque<T> {
    spine: Spine<T>,
    len: usize,
}

impl<T> Clone for CatenableDeque<T> {
    fn clone(&self) -> Self {
        CatenableDeque {
            spine: self.spine.clone(),
            len: self.len,
        }
    }
}

impl<T: 'static> Default for CatenableDeque<T> {
    fn default() -> Self {
        CatenableDeque {
            spine: Spine::empty(),
            len: 0,
        }
    }
}

impl<T: 'static + fmt::Debug> fmt::Debug for CatenableDeque<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl<T: 'static> CatenableDeque<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn wrap(spine: Spine<T>, len: usize) -> Self {
        CatenableDeque { spine, len }
    }

    fn unwrap_value(elem: Elem<T>) -> Ref<T> {
        match elem {
            Elem::Value(value) => value,
            Elem::Chunk(_) => unreachable!("the outermost level holds only values"),
        }
    }

    pub fn append(&self, other: &Self) -> Self {
        Self::wrap(self.spine.append(&other.spine), self.len + other.len)
    }
}

impl<T: 'static> Deque<T> for CatenableDeque<T> {
    type Iter = CatenableDequeIterator<T>;

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn len(&self) -> usize {
        self.len
    }

    fn push_front(&self, value: T) -> Self {
        Self::wrap(self.spine.cons(Elem::Value(Ref::new(value))), self.len + 1)
    }

    fn push_back(&self, value: T) -> Self {
        Self::wrap(self.spine.snoc(Elem::Value(Ref::new(value))), self.len + 1)
    }

    fn front(&self) -> Option<Ref<T>> {
        self.spine.head().map(Self::unwrap_value)
    }

    fn back(&self) -> Option<Ref<T>> {
        self.spine.last().map(Self::unwrap_value)
    }

    fn pop_front(&self) -> Option<(Ref<T>, Self)> {
        let value = self.front()?;
        Some((value, Self::wrap(self.spine.tail()?, self.len - 1)))
    }

    fn pop_back(&self) -> Option<(Ref<T>, Self)> {
        let value = self.back()?;
        Some((value, Self::wrap(self.spine.init()?, self.len - 1)))
    }

    fn iter(&self) -> Self::Iter {
        CatenableDequeIterator {
            stack: vec![Frame::Spine(self.spine.clone())],
        }
    }
}

// iteration

enum Frame<T> {
    Chunk(BankersDequeIterator<Elem<T>>),
    // A spine not yet expanded into chunk frames.
    Spine(Spine<T>),
}

pub struct CatenableDequeIterator<T> {
    stack: Vec<Frame<T>>,
}

impl<T: 'static> Iterator for CatenableDequeIterator<T> {
    type Item = Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.pop()? {
                Frame::Spine(spine) => match spine.node.as_ref() {
                    CatNode::Shallow(chunk) => self.stack.push(Frame::Chunk(chunk.iter())),
                    CatNode::Deep { head, middle, tail } => {
                        // pushed back-to-front so the stack pops them in order
                        self.stack.push(Frame::Chunk(tail.iter()));
                        self.stack.push(Frame::Spine(middle.force()));
                        self.stack.push(Frame::Chunk(head.iter()));
                    }
                },
                Frame::Chunk(mut chunk) => {
                    let Some(elem) = chunk.next() else { continue };
                    self.stack.push(Frame::Chunk(chunk));
                    match &*elem {
                        Elem::Value(value) => return Some(value.clone()),
                        Elem::Chunk(inner) => self.stack.push(Frame::Chunk(inner.iter())),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::empty::Empty;
    use std::collections::VecDeque;

    fn build(values: &[i32]) -> CatenableDeque<i32> {
        values
            .iter()
            .fold(CatenableDeque::empty(), |deque, &value| {
                deque.push_back(value)
            })
    }

    fn collect(deque: &CatenableDeque<i32>) -> Vec<i32> {
        deque.iter().map(|value| *value).collect()
    }

    fn assert_matches(deque: &CatenableDeque<i32>, model: &VecDeque<i32>) {
        assert_eq!(deque.len(), model.len(), "len mismatch");
        assert_eq!(deque.is_empty(), model.is_empty(), "is_empty mismatch");
        assert_eq!(collect(deque), model.iter().copied().collect::<Vec<_>>());
        assert_eq!(deque.front().map(|value| *value), model.front().copied());
        assert_eq!(deque.back().map(|value| *value), model.back().copied());
    }

    #[test]
    fn test_empty_deque() {
        let deque: CatenableDeque<i32> = CatenableDeque::empty();
        assert!(deque.is_empty());
        assert_eq!(deque.len(), 0);
        assert!(deque.front().is_none());
        assert!(deque.back().is_none());
        assert!(deque.pop_front().is_none());
        assert!(deque.pop_back().is_none());
    }

    #[test]
    fn test_push_front_reverses_order() {
        let deque = CatenableDeque::empty()
            .push_front(1)
            .push_front(2)
            .push_front(3);
        assert_eq!(collect(&deque), vec![3, 2, 1]);
        assert_eq!(*deque.front().unwrap(), 3);
        assert_eq!(*deque.back().unwrap(), 1);
        assert_eq!(deque.len(), 3);
    }

    #[test]
    fn test_push_back_keeps_order() {
        let deque = build(&[1, 2, 3]);
        assert_eq!(collect(&deque), vec![1, 2, 3]);
        assert_eq!(*deque.front().unwrap(), 1);
        assert_eq!(*deque.back().unwrap(), 3);
        assert_eq!(deque.len(), 3);
    }

    #[test]
    fn test_drain_from_front() {
        let mut deque = build(&(0..64).collect::<Vec<_>>());
        for expected in 0..64 {
            let (value, rest) = deque.pop_front().expect("still non-empty");
            assert_eq!(*value, expected);
            deque = rest;
        }
        assert!(deque.is_empty());
    }

    #[test]
    fn test_drain_from_back() {
        let mut deque = build(&(0..64).collect::<Vec<_>>());
        for expected in (0..64).rev() {
            let (value, rest) = deque.pop_back().expect("still non-empty");
            assert_eq!(*value, expected);
            deque = rest;
        }
        assert!(deque.is_empty());
    }

    #[test]
    fn test_front_and_back_do_not_consume() {
        let deque = build(&[10, 20, 30, 40]);
        assert_eq!(*deque.front().unwrap(), 10);
        assert_eq!(*deque.back().unwrap(), 40);
        assert_eq!(deque.len(), 4);
    }

    #[test]
    fn test_earlier_versions_are_unaffected() {
        let empty = CatenableDeque::empty();
        let one = empty.push_back(1);
        let two = one.push_back(2);

        assert!(empty.is_empty());
        assert_eq!(one.len(), 1);
        assert_eq!(two.len(), 2);

        let (_, popped) = two.pop_front().unwrap();
        assert_eq!(two.len(), 2);
        assert_eq!(popped.len(), 1);
    }

    #[test]
    fn test_append_basic() {
        let combined = build(&[1, 2]).append(&build(&[3, 4]));
        assert_eq!(collect(&combined), vec![1, 2, 3, 4]);
        assert_eq!(combined.len(), 4);
        assert_eq!(*combined.front().unwrap(), 1);
        assert_eq!(*combined.back().unwrap(), 4);
    }

    #[test]
    fn test_append_with_empty() {
        let deque = build(&[1, 2, 3]);
        let empty = CatenableDeque::empty();
        assert_eq!(collect(&empty.append(&deque)), vec![1, 2, 3]);
        assert_eq!(collect(&deque.append(&empty)), vec![1, 2, 3]);
        assert!(empty.append(&empty).is_empty());
    }

    #[test]
    fn test_append_is_associative() {
        let left = build(&[1, 2, 3, 4]);
        let middle = build(&[5, 6, 7, 8]);
        let right = build(&[9, 10, 11, 12]);
        let expected: Vec<i32> = (1..=12).collect();

        assert_eq!(collect(&left.append(&middle).append(&right)), expected);
        assert_eq!(collect(&left.append(&middle.append(&right))), expected);
    }

    #[test]
    fn test_deep_append_then_drain_both_ends() {
        let mut deque = build(&[0, 1, 2, 3]);
        let mut expected: Vec<i32> = vec![0, 1, 2, 3];
        for step in 1..200 {
            let chunk: Vec<i32> = (step * 4..step * 4 + 4).collect();
            deque = deque.append(&build(&chunk));
            expected.extend(chunk);
        }
        assert_eq!(collect(&deque), expected);

        let (mut front, mut back) = (0usize, expected.len());
        let mut current = deque;
        while front < back {
            let (value, rest) = current.pop_front().expect("non-empty");
            assert_eq!(*value, expected[front]);
            front += 1;
            current = rest;
            if front < back {
                let (value, rest) = current.pop_back().expect("non-empty");
                assert_eq!(*value, expected[back - 1]);
                back -= 1;
                current = rest;
            }
        }
        assert!(current.is_empty());
    }

    #[test]
    fn test_iterates_a_deep_structure() {
        // built by catenation, so the spine is actually populated
        let deque = build(&[10, 20, 30])
            .append(&build(&[40, 50, 60]))
            .append(&build(&[70, 80, 90, 100]));
        assert_eq!(
            collect(&deque),
            vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
        );
    }

    struct Xorshift(u64);

    impl Xorshift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, bound: usize) -> usize {
            (self.next() % bound as u64) as usize
        }
    }

    // Randomised differential test against `VecDeque`
    // Keeping a pool of versions alive so the structure is exercised persistently.
    #[test]
    fn test_matches_vecdeque_model() {
        const MAX_LEN: usize = 3000;
        let mut random = Xorshift(0x2545_F491_4F6C_DD1D);
        let mut pool: Vec<(CatenableDeque<i32>, VecDeque<i32>)> =
            vec![(CatenableDeque::empty(), VecDeque::new())];
        let mut next_value = 0i32;

        for _ in 0..4000 {
            let index = random.below(pool.len());
            let (deque, model) = pool[index].clone();

            let (new_deque, new_model) = match random.below(6) {
                0 => {
                    let value = next_value;
                    next_value += 1;
                    let mut model = model;
                    model.push_front(value);
                    (deque.push_front(value), model)
                }
                1 => {
                    let value = next_value;
                    next_value += 1;
                    let mut model = model;
                    model.push_back(value);
                    (deque.push_back(value), model)
                }
                2 => match deque.pop_front() {
                    None => {
                        assert!(model.is_empty());
                        continue;
                    }
                    Some((value, rest)) => {
                        let mut model = model;
                        assert_eq!(Some(*value), model.pop_front());
                        (rest, model)
                    }
                },
                3 => match deque.pop_back() {
                    None => {
                        assert!(model.is_empty());
                        continue;
                    }
                    Some((value, rest)) => {
                        let mut model = model;
                        assert_eq!(Some(*value), model.pop_back());
                        (rest, model)
                    }
                },
                _ => {
                    let other_index = random.below(pool.len());
                    let (other_deque, other_model) = pool[other_index].clone();
                    if model.len() + other_model.len() > MAX_LEN {
                        continue;
                    }
                    let mut model = model;
                    model.extend(other_model.iter().copied());
                    (deque.append(&other_deque), model)
                }
            };

            assert_matches(&new_deque, &new_model);

            if pool.len() < 40 {
                pool.push((new_deque, new_model));
            } else {
                let victim = random.below(pool.len());
                pool[victim] = (new_deque, new_model);
            }
        }
    }
}
