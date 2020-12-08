use std::{io::Read, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}};

use client::spawn_client;

extern crate modular_core;
extern crate anyhow;
extern crate rosc;

pub mod client;
pub mod osc;
