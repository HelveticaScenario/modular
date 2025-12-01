use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use modular_server::protocol::{InputMessage, OutputMessage};
use std::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub fn spawn_client(
    server_url: String,
    incoming_tx: Sender<OutputMessage>,
    outgoing_rx: Receiver<InputMessage>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let ws_url = format!("ws://{}/ws", server_url);
    
    // Spawn WebSocket sending task
    let send_handle = tokio::spawn({
        let ws_url = ws_url.clone();
        async move {
            if let Err(e) = websocket_send_loop(ws_url, outgoing_rx).await {
                eprintln!("WebSocket send error: {}", e);
            }
        }
    });
    
    // Spawn WebSocket receiving task
    let recv_handle = tokio::spawn(async move {
        if let Err(e) = websocket_recv_loop(ws_url, incoming_tx).await {
            eprintln!("WebSocket receive error: {}", e);
        }
    });
    
    (recv_handle, send_handle)
}

async fn websocket_send_loop(ws_url: String, outgoing_rx: Receiver<InputMessage>) -> Result<()> {
    loop {
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                println!("Connected to WebSocket at {}", ws_url);
                let (mut write, _read) = ws_stream.split();
                
                // Process messages from the channel
                loop {
                    // Check for messages from the channel (non-blocking via try_recv)
                    match outgoing_rx.try_recv() {
                        Ok(message) => {
                            let yaml = serde_yaml::to_string(&message)?;
                            if write.send(Message::Text(yaml)).await.is_err() {
                                eprintln!("Failed to send message, reconnecting...");
                                break;
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // No messages, sleep briefly
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            println!("Outgoing channel disconnected");
                            return Ok(());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to WebSocket: {}. Retrying in 2s...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn websocket_recv_loop(ws_url: String, incoming_tx: Sender<OutputMessage>) -> Result<()> {
    loop {
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                println!("Connected to WebSocket for receiving at {}", ws_url);
                let (_write, mut read) = ws_stream.split();
                
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // Try YAML first, then JSON for backward compatibility
                            let output_msg: Result<OutputMessage, _> = serde_yaml::from_str(&text)
                                .or_else(|_| serde_json::from_str(&text));
                            
                            match output_msg {
                                Ok(msg) => {
                                    if incoming_tx.send(msg).is_err() {
                                        println!("Incoming channel disconnected");
                                        return Ok(());
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse message: {}", e);
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            println!("WebSocket closed by server");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to WebSocket: {}. Retrying in 2s...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}
