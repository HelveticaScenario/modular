use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex as StdMutex},
    thread::{self, JoinHandle},
};
use modular_core::crossbeam_channel::{Receiver, Sender};

use modular_core::message::{InputMessage, OutputMessage};

pub fn start_sending_server(rx: Receiver<OutputMessage>, clients: Arc<StdMutex<Vec<TcpStream>>>) {
    for message in rx {
        let json = serde_json::to_string(&message).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "Error",
                "message": "Failed to serialize message"
            })
            .to_string()
        });
        let json_line = format!("{}\n", json);
        
        let mut clients_lock = clients.lock().unwrap();
        clients_lock.retain_mut(|client| {
            match client.write_all(json_line.as_bytes()) {
                Ok(_) => true,
                Err(_) => {
                    println!("Client disconnected");
                    false
                }
            }
        });
    }
}

pub fn start_receiving_server(host_address: String, tx: Sender<InputMessage>, clients: Arc<StdMutex<Vec<TcpStream>>>) {
    let listener = TcpListener::bind(&host_address).unwrap();
    println!("JSON API listening on {}", host_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New client connected: {:?}", stream.peer_addr());
                
                // Add client to broadcast list
                {
                    let mut clients_lock = clients.lock().unwrap();
                    clients_lock.push(stream.try_clone().unwrap());
                }

                let tx = tx.clone();
                thread::spawn(move || {
                    handle_client(stream, tx);
                });
            }
            Err(e) => {
                println!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_client(stream: TcpStream, tx: Sender<InputMessage>) {
    let reader = BufReader::new(stream);
    
    for line in reader.lines() {
        match line {
            Ok(json) => {
                if json.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<InputMessage>(&json) {
                    Ok(message) => {
                        if let Err(e) = tx.send(message) {
                            println!("Error sending message to modular core: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Error parsing JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Client disconnected: {}", e);
                break;
            }
        }
    }
}

pub fn spawn_server(
    _client_address: String,
    server_port: String,
    tx: Sender<InputMessage>,
    rx: Receiver<OutputMessage>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let host_address = format!("127.0.0.1:{}", server_port);
    let clients = Arc::new(StdMutex::new(Vec::new()));
    
    let receiving_server_handle = {
        let clients = clients.clone();
        thread::spawn(move || start_receiving_server(host_address, tx, clients))
    };
    
    let sending_server_handle = {
        thread::spawn(move || start_sending_server(rx, clients))
    };

    (receiving_server_handle, sending_server_handle)
}
