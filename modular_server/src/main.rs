extern crate anyhow;
extern crate clap;
extern crate ctrlc;
extern crate modular_core;
extern crate rosc;
use std::{fs::File, io::Read, sync::mpsc};

use clap::{App, Arg, ArgMatches};
use modular_core::Modular;
use server::{spawn_server};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use stringreader::StringReader;

mod osc;
mod server;

fn main() -> anyhow::Result<()> {
    let matches = get_matches();

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let config_file: Box<dyn Read> = if let Some(config) = matches.value_of(CONFIG_ARG) {
        Box::new(File::open(config)?)
    } else {
        Box::new(StringReader::new("{}"))
    };
    let modular = Modular::new(serde_json::from_reader(config_file)?)?;
    let modular_handle = modular.spawn(incoming_rx, outgoing_tx);
    let running = Arc::new(AtomicBool::new(true));
    let client_address = matches.value_of(CLIENT_ARG).unwrap();
    let port = matches.value_of(PORT_ARG).unwrap();

    let (recieving_server_handle, sending_server_handle) = spawn_server(
        client_address.to_owned(),
        port.to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}

    Ok(())
}

const CONFIG_ARG: &str = "config";
const CLIENT_ARG: &str = "client";
const PORT_ARG: &str = "port";

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("Modular")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name(CONFIG_ARG)
                .short("c")
                .long(CONFIG_ARG)
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(CLIENT_ARG)
                .long(CLIENT_ARG)
                .value_name("IP_ADDRESS")
                .default_value("127.0.0.1:7813")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(PORT_ARG)
                .long(PORT_ARG)
                .value_name("PORT")
                .default_value("7812")
                .takes_value(true),
        )
        .get_matches()
}