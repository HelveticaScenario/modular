// use std::{
//     net::SocketAddrV4,
//     net::UdpSocket,
//     str::FromStr,
//     thread::{self, JoinHandle},
// };
// use modular_core::crossbeam_channel::{Sender, Receiver};

// use modular_core::message::{InputMessage, OutputMessage};
// use rosc::encoder;

// use crate::osc::{message_to_osc, osc_to_message};

// pub fn start_sending_server(client_address: String, rx: Receiver<OutputMessage>) {
//     let host_addr = SocketAddrV4::from_str("0.0.0.0:0").unwrap();
//     let to_addr = SocketAddrV4::from_str(&client_address).unwrap();
//     println!("Sending to {}", to_addr);
//     let sock = UdpSocket::bind(host_addr).unwrap();

//     for message in rx {
//         for packet in message_to_osc(message) {
//             let msg_buf = encoder::encode(&packet).unwrap();
//             sock.send_to(&msg_buf, to_addr).unwrap();
//         }
//     }
// }

// pub fn start_recieving_server(host_address: String, tx: Sender<InputMessage>) {
//     let addr = SocketAddrV4::from_str(&host_address).unwrap();
//     let sock = UdpSocket::bind(addr).unwrap();
//     println!("Listening to {}", addr);

//     let mut buf = [0u8; rosc::decoder::MTU];

//     loop {
//         match sock.recv_from(&mut buf) {
//             Ok((size, _addr)) => match rosc::decoder::decode(&buf[..size]) {
//                 Ok(packet) => {
//                     // println!("{:?}", packet);
//                     osc_to_message(packet, &tx)
//                 }
//                 Err(err) => {
//                     println!("{:?}", err);
//                 }
//             },
//             Err(e) => {
//                 println!("Error receiving from socket: {}", e);
//                 return;
//             }
//         }
//     }
// }

// pub fn spawn_server(
//     client_address: String,
//     server_port: String,
//     tx: Sender<InputMessage>,
//     rx: Receiver<OutputMessage>,
// ) -> (JoinHandle<()>, JoinHandle<()>) {
//     let host_address = format!("127.0.0.1:{}", server_port);
//     let recieving_server_handle = {
//         let host_address = host_address.clone();
//         thread::spawn(move || start_recieving_server(host_address, tx))
//     };
//     let sending_server_handle = thread::spawn(move || start_sending_server(client_address, rx));

//     (recieving_server_handle, sending_server_handle)
// }
