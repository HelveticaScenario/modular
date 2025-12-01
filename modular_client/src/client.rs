use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle, sleep},
    time::Duration,
};

use modular_server::protocol::{InputMessage, OutputMessage};

pub fn start_sending_client(server_address: String, rx: Receiver<InputMessage>) {
    loop {
        match TcpStream::connect(&server_address) {
            Ok(mut stream) => {
                println!("Connected to server at {}", server_address);
                
                for message in &rx {
                    let yaml = match serde_yaml::to_string(&message) {
                        Ok(y) => y,
                        Err(e) => {
                            println!("Failed to serialize message: {}", e);
                            continue;
                        }
                    };
                    
                    if let Err(e) = stream.write_all(yaml.as_bytes()) {
                        println!("Failed to send message: {}", e);
                        break;
                    }
                }
                break;
            }
            Err(e) => {
                println!("Failed to connect to server: {}. Retrying in 2s...", e);
                sleep(Duration::from_secs(2));
            }
        }
    }
}

pub fn start_receiving_client(server_address: String, tx: Sender<OutputMessage>) {
    loop {
        match TcpStream::connect(&server_address) {
            Ok(stream) => {
                println!("Connected to server for receiving at {}", server_address);
                let reader = BufReader::new(stream);
                
                for line in reader.lines() {
                    match line {
                        Ok(yaml) => {
                            if yaml.trim().is_empty() {
                                continue;
                            }
                            match serde_yaml::from_str::<OutputMessage>(&yaml) {
                                Ok(message) => {
                                    if let Err(e) = tx.send(message) {
                                        println!("Error sending message to handler: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("Error parsing YAML: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Connection lost: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to connect to server: {}. Retrying in 2s...", e);
                sleep(Duration::from_secs(2));
            }
        }
    }
}

pub fn spawn_client(
    server_address: String,
    _client_port: String,
    tx: Sender<OutputMessage>,
    rx: Receiver<InputMessage>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let receiving_client_handle = {
        let server_address = server_address.clone();
        thread::spawn(move || start_receiving_client(server_address, tx))
    };
    let sending_client_handle = thread::spawn(move || start_sending_client(server_address, rx));

    (receiving_client_handle, sending_client_handle)
}
