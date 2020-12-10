use std::{cell::RefCell, mem, rc::Rc};

struct Node {
    next_node: Box<Option<Node>>,
    value: usize,
}

impl Node {
    fn insert(&mut self, value: usize) {
        if value > self.value {
            let mut tmp: Box<Option<Node>> = Box::new(None);
            mem::swap(&mut self.next_node, &mut tmp);
            match *tmp {
                Some(mut node) => {
                    if value > node.value {
                        node.insert(value);
                    } else if value < node.value {
                        node = Node {
                            next_node: Box::new(Some(node)),
                            value,
                        }
                    }
                    self.next_node = Box::new(Some(node));
                }
                None => {}
            }
        }
    }
}

pub struct Sequence {
    head: Box<Option<Node>>,
}

impl Sequence {
    fn new(value: usize) -> Self {
        Sequence {
            head: Box::new(Some(Node {
                value,
                next_node: Box::new(None),
            })),
        }
    }
    fn next(&mut self) -> usize {
        let mut tmp = Box::new(None);
        mem::swap(&mut self.head, &mut tmp);
        let mut head = tmp.unwrap();
        let value = head.value;
        let next = head.next_node;
        self.head = match *next {
            Some(node) => Box::new(Some(node)),
            None => Box::new(Some(Node {
                value: value + 1,
                next_node: Box::new(None),
            })),
        };
        value
    }
    fn reclaim(&mut self, value: usize) {
        let mut tmp = Box::new(None);
        mem::swap(&mut self.head, &mut tmp);
        let mut head = tmp.unwrap();
        head.insert(value);
        self.head = Box::new(Some(head));
    }
}
