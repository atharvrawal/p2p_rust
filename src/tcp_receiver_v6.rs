use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) {
    let peer_addr = stream.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap());
    let mut buf = [0u8; 1024];

    match stream.read(&mut buf) {
        Ok(len) if len > 0 => {
            let msg = String::from_utf8_lossy(&buf[..len]);
            println!("Received from [{}]: {}", peer_addr, msg);
        }
        Ok(_) => {
            println!("Connection from [{}] closed without data", peer_addr);
        }
        Err(e) => eprintln!("Failed to read from [{}]: {}", peer_addr, e),
    }
}

fn main() {
    let listener = TcpListener::bind("[::]:42070").expect("Failed to bind TCP listener");
    println!("📡 Listening on [::]:42070 (IPv6 TCP)");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}
