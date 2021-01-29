use client::spawn_client;
use futures::{FutureExt, StreamExt};
use modular_server::spawn;
use modular_core::crossbeam_channel::unbounded;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod client;
mod osc;

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

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


    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let assets = warp::path("assets").and(warp::fs::dir("./modular_web/client/dist"));

    let ws = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|websocket| {

        }));

    warp::serve(index.or(assets).or(ws))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

static INDEX_HTML: &str = include_str!("../client/src/index.html");
