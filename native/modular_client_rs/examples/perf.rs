use std::{collections::HashMap, rc::Rc, time::Instant};

use modular_core::uuid::Uuid;

pub fn main() {
    {
        let mut h = HashMap::new();
        let u = Uuid::new_v4();
        h.insert(u.clone(), 1);
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for _ in 0..1000 {
                a += h.get(&u).unwrap();
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    {
        let mut h = HashMap::new();
        let u = Uuid::new_v4().to_string();
        h.insert(u.clone(), 1);
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for _ in 0..1000 {
                a += h.get(&u).unwrap();
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    {
        let mut h = HashMap::new();
        h.insert(0, 1);
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for _ in 0..1000 {
                a += h.get(&0).unwrap();
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    {
        let mut h = HashMap::new();
        for i in 0..1000 {
            let u = Uuid::new_v4();
            h.insert(u, i);
        }
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for (_, i) in h.iter() {
                a += i;
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    {
        let mut h = vec![];
        for i in 0..1000 {
            h.push(i);
        }
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for i in 0..1000 {
                a += h.get(i).unwrap();
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    {
        let mut h = HashMap::new();
        for i in 0..1000 {
            let u = Uuid::new_v4().to_string();
            h.insert(u, i);
        }
        for _ in 0..4 {
            let now = Instant::now();
            let mut a = 0;
            for (_, i) in h.iter() {
                a += i;
            }
            let e = now.elapsed();
            println!("{:?} {}", e, a);
        }
    }
    println!("");
    let hh = Rc::new(1);
    let hw = Rc::downgrade(&hh);
    for _ in 0..4 {
        let now = Instant::now();
        let mut a = 0;
        for _ in 0..1000 {
            match hw.upgrade() {
                Some(i) => {
                    a += *i;
                }
                None => {
                    a += 0;
                }
            }
        }
        let e = now.elapsed();
        println!("{:?} {}", e, a);
    }
    println!("");
    drop(hh);
    for _ in 0..4 {
        let now = Instant::now();
        let mut a = 0;
        for _ in 0..1000 {
            match hw.upgrade() {
                Some(i) => {
                    a += *i;
                }
                None => {
                    a += 0;
                }
            }
        }
        let e = now.elapsed();
        println!("{:?} {}", e, a);
    }
}
