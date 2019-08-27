extern crate ctrlc;
extern crate websocket;
extern crate serde_json;

use serde_json::{Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::sync::mpsc::channel;
use websocket::{Message, OwnedMessage};
use websocket::client::ClientBuilder;


fn main() {
    // note: reconnection handling needed.
    let client = ClientBuilder::new("wss://ws.lightstream.bitflyer.com/json-rpc")
        .unwrap()
        .add_protocol("json-rpc")
        .connect_insecure()
        .unwrap();
    let (mut receiver, mut sender) = client.split().unwrap();
   	let (tx, rx) = channel();
	let tx_1 = tx.clone();

    // Sender section
	let send_loop = thread::spawn(move || {
		loop {
			// Send loop
			let message = match rx.recv() {
				Ok(m) => m,
				Err(e) => {
					println!("Got error in send loop: {:?}", e);
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					let _ = sender.send_message(&message);
					return;
				}
				_ => (),
			}
			// Send the message
			match sender.send_message(&message) {
				Ok(()) => (),
				Err(e) => {
					println!("Failed to send message: {:?}", e);
					let _ = sender.send_message(&Message::close());
					return;
				}
			}
		}
	});

    // Reciver section
	let receive_loop = thread::spawn(move || {
		for message in receiver.incoming_messages() {
			let message = match message {
				Ok(m) => m,
				Err(e) => {
					println!("Got error in receive loop: {:?}", e);
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					// Got a close message, so send a close message and return
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				}
				OwnedMessage::Ping(data) => {
					match tx_1.send(OwnedMessage::Pong(data)) {
						// Send a pong in response
						Ok(()) => (),
						Err(e) => {
							println!("Receive Loop: {:?}", e);
							return;
						}
					}
				}

                // msg typeで必要に応じ分岐
				OwnedMessage::Text(message) => {
					let v: Value = serde_json::from_str(&message).unwrap();
					if v["jsonrpc"] == "2.0" && v["method"] == "lightning_executions_FX_BTC_JPY" {
						println!("{} {}", v["params"]["channel"], v["params"]["message"]);
					}
				}
				_ => {
					// ignore.
				},
			}
		}
	});

    // r#""# is string escape format
    let send_msg: String = r#"{"jsonrpc":"2.0","method":"subscribe","params":{"channel":"lightning_executions_FX_BTC_JPY"}}"#.to_string();
    println!("{}",send_msg);
	match tx.send(OwnedMessage::Text(send_msg)) {
		Ok(()) => (),
		Err(e) => {
			println!("Failed to send subscribe request: {:?}", e);
		}
	}
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    println!("Waiting for Ctrl-C...");
    while running.load(Ordering::SeqCst) {}

    // start send/raceive routine
	let _ = send_loop.join();
	let _ = receive_loop.join();
}