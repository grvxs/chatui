use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::time::Duration;
use std::{process, thread};

use serde_json::json;
use serde_json::Value;

const LOCAL: &str = "127.0.0.1:3000";
const MSG_SIZE: usize = 256;

fn get_name() -> String {
    println!("Enter your name:");
    let mut name = String::new();
    io::stdin()
        .read_line(&mut name)
        .expect("Reading from stdin failed.");
    name.trim().to_string()
}

fn start_tx_loop(tx: Sender<String>) {
    loop {
        let mut buff = String::new();
        io::stdin()
            .read_line(&mut buff)
            .expect("Reading from stdin failed.");
        if tx.send(buff.trim().to_string()).is_err() {
            break;
        }
    }
}

fn parse(buff: Vec<u8>) {
    let msg = String::from_utf8(
        buff.into_iter()
            .take_while(|&x| x != 0)
            .collect::<Vec<u8>>(),
    )
    .expect("Invalid utf8 message.");
    let data: Value = serde_json::from_str(&msg).expect("Failed to parse data.");
    println!(
        "{}: {}",
        data["name"].as_str().unwrap(),
        data["message"].as_str().unwrap()
    );
}

fn start_rx_loop(name: String) -> (Sender<String>, std::thread::JoinHandle<()>) {
    let mut client = TcpStream::connect(LOCAL).expect("Stream failed to connect.");

    client
        .set_nonblocking(true)
        .expect("Failed to initiate non-blocking.");

    let (tx, rx) = mpsc::channel::<String>();

    let handle = thread::spawn(move || loop {
        let mut buff = vec![0; MSG_SIZE];

        match client.read_exact(&mut buff) {
            Ok(_) => {
                parse(buff);
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => {}
            Err(_) => {
                println!("Server stopped responding.");
                process::exit(1);
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = json!({ "name": name, "message": msg })
                    .to_string()
                    .into_bytes();
                buff.resize(MSG_SIZE, 0);
                client.write_all(&buff).expect("Writing to socket failed.");
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        thread::sleep(Duration::from_millis(100));
    });

    (tx, handle)
}

fn main() {
    let name = get_name();

    let (tx, _handle) = start_rx_loop(name);

    start_tx_loop(tx);

    println!("Bye.");
}
