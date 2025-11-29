extern crate anyhow;
extern crate clap;
extern crate ctrlc;
extern crate modular_core;

use clap::{Arg, ArgMatches, Command};
use modular_server::spawn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let matches = get_matches();

    let running = Arc::new(AtomicBool::new(true));
    let client_address = matches.get_one::<String>(CLIENT_ARG).unwrap();
    let port = matches.get_one::<String>(PORT_ARG).unwrap();

    let (_modular_handle, _receiving_server_handle, _sending_server_handle) =
        spawn(client_address.to_owned(), port.to_owned());
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}
}

const CLIENT_ARG: &str = "client";
const PORT_ARG: &str = "port";

fn get_matches() -> ArgMatches {
    Command::new("Modular")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new(CLIENT_ARG)
                .long(CLIENT_ARG)
                .value_name("IP_ADDRESS")
                .default_value("127.0.0.1:7813")
                .num_args(1),
        )
        .arg(
            Arg::new(PORT_ARG)
                .long(PORT_ARG)
                .value_name("PORT")
                .default_value("7812")
                .num_args(1),
        )
        .get_matches()
}
