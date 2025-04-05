use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use std::fs::File;
use std::str::from_utf8;
use std::io::Write;
use bincode;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    pub header: u64,
    pub sno: u32,
    pub payload_length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind server");
    println!("Waiting for connection...");

    let (mut stream, _) = listener.accept().await.expect("Failed to accept connection");
    println!("Client connected!");

    let mut packets = Vec::new();

    loop {
        // First read the packet length (4 bytes)
        let mut len_bytes = [0u8; 4];
        if let Err(e) = stream.read_exact(&mut len_bytes).await {
            println!("Connection closed or error: {}", e);
            break;
        }
        
        let packet_len = u32::from_be_bytes(len_bytes) as usize;
        
        // Then read the actual packet data
        let mut packet_buffer = vec![0; packet_len];
        if let Err(e) = stream.read_exact(&mut packet_buffer).await {
            println!("Error reading packet: {}", e);
            break;
        }

        let packet: Packet = bincode::deserialize(&packet_buffer).expect("Failed to deserialize");
        packets.push(packet);
    }

    if !packets.is_empty() {
        packets_to_file(packets);
        println!("File received and saved successfully!");
    } else {
        println!("No packets received.");
    }
}

fn packets_to_file(packets: Vec<Packet>) {
    let file_name = from_utf8(&packets[0].payload).unwrap();
    let mut file = File::create(file_name).expect("Failed to create file");

    for packet in &packets[1..] {
        file.write_all(&packet.payload).expect("Failed to write data");
    }

    println!("File '{}' reconstructed successfully!", file_name);
}