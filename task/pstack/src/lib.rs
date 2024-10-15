#![forbid(unsafe_code)]

use std::rc::Rc;

struct Node<T> {
    data: Rc<T>,
    next: Option<Rc<Node<T>>>,
}

pub struct PStack<T> {
    head: Option<Rc<Node<T>>>,
    size: usize,
}

impl<T> Default for PStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for PStack<T> {
    fn clone(&self) -> Self {
        PStack {
            head: self.head.clone(),
            size: self.size,
        }
    }
}

impl<T> PStack<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            size: 0,
        }
    }

    pub fn push(&self, value: T) -> Self {
        PStack {
            head: Some(Rc::new(Node {
                data: Rc::new(value),
                next: self.head.clone(),
            })),
            size: self.size + 1,
        }
    }

    pub fn pop(&self) -> Option<(Rc<T>, Self)> {
        self.head.as_ref().map(|head| {
            (
                Rc::clone(&head.data),
                PStack {
                    head: head.next.clone(),
                    size: self.size - 1,
                },
            )
        })
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = Rc<T>> {
        PStackIterator {
            current: self.head.clone(),
        }
    }
}

struct PStackIterator<T> {
    current: Option<Rc<Node<T>>>,
}

impl<T> Iterator for PStackIterator<T> {
    type Item = Rc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current.clone() {
            Some(head) => {
                self.current = head.next.clone();
                Some(head.data.clone())
            }
            None => None,
        }
    }
}
