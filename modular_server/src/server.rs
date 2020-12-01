use std::{
    net::SocketAddrV4,
    net::UdpSocket,
    str::FromStr,
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};

use modular_core::message::Message;
use rosc::{encoder, OscMessage, OscPacket, OscType};

use crate::osc::{message_to_osc, osc_to_message};

pub fn start_sending_server(client_address: String, rx: Receiver<Message>) {
    let host_addr = SocketAddrV4::from_str("0.0.0.0:0").unwrap();
    let to_addr = SocketAddrV4::from_str(&client_address).unwrap();
    let sock = UdpSocket::bind(host_addr).unwrap();

    for message in rx {
        let msg_buf = encoder::encode(&message_to_osc(message)).unwrap();
        sock.send_to(&msg_buf, to_addr).unwrap();
    }

    // // send random values to xy fields
    // &OscPacket::Message(OscMessage {
    //     addr: "/3".to_string(),
    //     args: vec![],
    // })
    // let steps = 128;
    // let step_size: f32 = 2.0 * f32::consts::PI / steps as f32;
    // for i in 0.. {
    //     let x = 0.5 + (step_size * (i % steps) as f32).sin() / 2.0;
    //     let y = 0.5 + (step_size * (i % steps) as f32).cos() / 2.0;
    //     let mut msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
    //         addr: "/3/xy1".to_string(),
    //         args: vec![OscType::Float(x), OscType::Float(y)],
    //     }))
    //     .unwrap();

    //     sock.send_to(&msg_buf, to_addr).unwrap();
    //     msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
    //         addr: "/3/xy2".to_string(),
    //         args: vec![OscType::Float(y), OscType::Float(x)],
    //     }))
    //     .unwrap();
    //     sock.send_to(&msg_buf, to_addr).unwrap();
    //     thread::sleep(Duration::from_millis(20));
    // }
}

pub fn start_recieving_server(host_address: String, tx: Sender<Message>) {
    let addr = SocketAddrV4::from_str(&host_address).unwrap();
    let sock = UdpSocket::bind(addr).unwrap();
    println!("Listening to {}", addr);

    let mut buf = [0u8; rosc::decoder::MTU];

    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, addr)) => {
                println!("Received packet with size {} from: {}", size, addr);
                let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                if let Some(message) = osc_to_message(packet) {
                    tx.send(message);
                }
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                return;
            }
        }
    }
}

// fn handle_packet(packet: OscPacket, tx: &Sender<Message>) {
//     match packet {
//         OscPacket::Message(msg) => {
//             println!("OSC address: {}", msg.addr);
//             println!("OSC arguments: {:?}", msg.args);
//         }
//         OscPacket::Bundle(bundle) => {
//             println!("OSC Bundle: {:?}", bundle);
//         }
//     }
// }

pub fn spawn_server(
    client_address: String,
    server_port: String,
    tx: Sender<Message>,
    rx: Receiver<Message>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let host_address = format!("127.0.0.1:{}", server_port);
    let recieving_server_handle = {
        let host_address = host_address.clone();
        thread::spawn(move || start_recieving_server(host_address, tx))
    };
    let sending_server_handle = thread::spawn(move || start_sending_server(client_address, rx));

    (recieving_server_handle, sending_server_handle)
}
