use crate::Ref;

enum ListNode<T> {
    Empty,
    Value {
        value: Ref<T>,
        next_node: Ref<ListNode<T>>,
    },
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ListNode<T> {
    fn clone(&self) -> Self {
        match self {
            ListNode::Empty => ListNode::Empty,
            ListNode::Value { value, next_node } => ListNode::Value {
                value: value.clone(),
                next_node: next_node.clone(),
            },
        }
    }
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        List {
            head: self.head.clone(),
            len: self.len,
        }
    }
}

pub struct ListIterator<T> {
    current: Ref<ListNode<T>>,
}

impl<T> Iterator for ListIterator<T> {
    type Item = Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current.as_ref() {
            ListNode::Empty => None,
            ListNode::Value { value, next_node } => {
                let return_value = value.clone();
                self.current = next_node.clone();
                Some(return_value)
            }
        }
    }
}

pub struct List<T> {
    head: Ref<ListNode<T>>,
    len: usize,
}

impl<T> List<T> {
    pub fn iter(&self) -> ListIterator<T> {
        ListIterator {
            current: self.head.clone(),
        }
    }

    pub fn split(&self) -> (List<T>, List<T>) {
        let mut first = List::<T>::new();
        let mut second = List::<T>::new();
        let mut current = self.clone();
        let half = self.length() / 2;
        let other_half = self.length() - half;
        for _ in 0..half {
            let (value_rc, new_list) = current.pop_front_rc().unwrap();
            first = first.push_front_rc(value_rc);
            current = new_list;
        }
        for _ in 0..other_half {
            let (value_rc, new_list) = current.pop_front_rc().unwrap();
            second = second.push_front_rc(value_rc);
            current = new_list;
        }
        (first.reverse(), second.reverse())
    }

    pub fn reverse(&self) -> List<T> {
        let mut node = self.head.clone();
        let mut last_node = Ref::new(ListNode::Empty);
        while let ListNode::Value { value, next_node } = node.as_ref() {
            let new_node = ListNode::Value {
                value: value.clone(),
                next_node: last_node,
            };
            last_node = Ref::new(new_node);
            node = next_node.clone();
        }
        List {
            head: last_node,
            len: self.len,
        }
    }

    pub fn new() -> List<T> {
        List {
            head: Ref::new(ListNode::Empty),
            len: 0,
        }
    }

    fn push_front_rc(&self, rc_value: Ref<T>) -> List<T> {
        List {
            head: Ref::new(ListNode::Value {
                value: rc_value,
                next_node: self.head.clone(),
            }),
            len: self.len + 1,
        }
    }

    pub fn push_front(&self, value: T) -> List<T> {
        self.push_front_rc(Ref::new(value))
    }

    pub fn is_empty(&self) -> bool {
        self.length() == 0
    }

    pub fn length(&self) -> usize {
        self.len
    }

    pub fn pop_front_rc(&self) -> Option<(Ref<T>, List<T>)> {
        match self.head.as_ref() {
            ListNode::Empty => Option::None,
            ListNode::Value {
                value,
                ref next_node,
            } => Option::Some((
                value.clone(),
                List {
                    head: next_node.clone(),
                    len: self.len - 1,
                },
            )),
        }
    }

    pub fn pop_front(&self) -> Option<(&T, List<T>)> {
        match self.head.as_ref() {
            ListNode::Empty => Option::None,
            ListNode::Value {
                value,
                ref next_node,
            } => Option::Some((
                value,
                List {
                    head: next_node.clone(),
                    len: self.len - 1,
                },
            )),
        }
    }

    pub fn front(&self) -> Option<&T> {
        self.pop_front().map(|(e, _)| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter() {
        let l = List::new()
            .push_front(4)
            .push_front(3)
            .push_front(2)
            .push_front(1);
        let v = vec![1, 2, 3, 4];
        for (idx, val) in l.iter().enumerate() {
            assert_eq!(v[idx], *val);
        }
    }

    #[test]
    fn test_split() {
        let l = List::new()
            .push_front(4)
            .push_front(3)
            .push_front(2)
            .push_front(1);
        let (a, b) = l.split();
        assert_eq!(a.length(), 2);
        assert_eq!(b.length(), 2);
    }

    #[test]
    fn test_list() {
        let empty_list = List::new();
        assert_eq!(empty_list.length(), 0);
        assert!(empty_list.is_empty());
        assert!(empty_list.pop_front().is_none());
        assert!(empty_list.front().is_none());

        let list = empty_list.push_front(123).push_front(987);
        assert_eq!(list.length(), 2);
        assert!(!list.is_empty());
        assert_eq!(list.front(), Some(&987));

        let (popped_element, remaining_list) = list.pop_front().unwrap();
        assert_eq!(*popped_element, 987);
        assert_eq!(remaining_list.length(), 1);
        assert_eq!(remaining_list.front(), Some(&123));
    }

    #[test]
    fn test_list_reverse() {
        let list = List::new().push_front(1).push_front(2);
        let reversed_list = list.reverse();

        let (first_element, list_after_first_pop) = reversed_list.pop_front().unwrap();
        assert_eq!(*first_element, 1);

        let (second_element, _) = list_after_first_pop.pop_front().unwrap();
        assert_eq!(*second_element, 2);
    }
}
