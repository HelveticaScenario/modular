use client::spawn_client;
use futures::StreamExt;
use modular_core::{crossbeam_channel::unbounded, message::InputMessage};
use modular_server::spawn;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    RwLock,
};
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod client;
mod osc;
type Outgoing = Arc<UnboundedSender<InputMessage>>;
type IncomingMap = Arc<RwLock<HashMap<usize, UnboundedSender<osc::Message>>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
#[tokio::main]
async fn main() {
    let (_modular_handle, _receiving_server_handle, _sending_server_handle) =
        spawn("127.0.0.1:7813".to_owned(), "7812".to_owned());

    let (incoming_tx, incoming_rx) = unbounded();
    let (outgoing_tx, outgoing_rx) = unbounded();
    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        "7813".to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    let outgoing: Outgoing = {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(_) = outgoing_tx.send(msg) {
                    break;
                }
            }
        });
        Arc::new(tx)
    };
    let mut incoming = {
        let (tx, rx) = mpsc::unbounded_channel();
        thread::spawn(move || {
            while let Ok(msg) = incoming_rx.recv() {
                if let Err(_) = tx.send(msg) {
                    break;
                }
            }
        });
        rx
    };

    let incoming_map: IncomingMap = Arc::new(RwLock::new(HashMap::new()));

    let connections = {
        let incoming_map = incoming_map.clone();
        let outgoing = outgoing.clone();
        warp::any().map(move || (outgoing.clone(), incoming_map.clone()))
    };

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let assets = warp::path("assets").and(warp::fs::dir("./modular_web/client/dist"));

    let ws = warp::path("ws").and(warp::ws()).and(connections).map(
        |ws: warp::ws::Ws, (outgoing, incoming_map)| {
            ws.on_upgrade(move |websocket| on_websocket(websocket, incoming_map, outgoing))
        },
    );

    tokio::task::spawn(async move {
        while let Some(msg) = incoming.recv().await {
            for sender in incoming_map.try_read_for(Duration::from_millis(10)).unwrap().await.values() {
                sender.send(msg.clone()).unwrap();
            }
        }
    });
    warp::serve(index.or(assets).or(ws))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

#[derive(Debug, Clone)]
enum Command {
    Incoming(Message),
    Outgoing(osc::Message),
}

async fn on_websocket(websocket: WebSocket, incoming_map: IncomingMap, outgoing: Outgoing) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new user: {}", my_id);

    let (user_ws_tx, mut user_ws_rx) = websocket.split();

    let (tx, mut rx) = mpsc::unbounded_channel();
    incoming_map.try_write_for(Duration::from_millis(10)).unwrap().await.insert(my_id, tx);

    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    
    let command_tx = Arc::new(command_tx);
    {
        let command_tx = command_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                command_tx.send(Command::Outgoing(msg)).unwrap();
            }
        });
    }
    tokio::spawn(async move {
        while let Some(Ok(msg)) = user_ws_rx.next().await {
            command_tx.send(Command::Incoming(msg)).unwrap();
        }
    });

    while let Some(command) = command_rx.recv().await {}
    incoming_map.try_write_for(Duration::from_millis(10)).unwrap().await.remove(&my_id);
}

static INDEX_HTML: &str = include_str!("../client/src/index.html");
