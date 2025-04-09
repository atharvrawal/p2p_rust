use tokio::net::UdpSocket;
use std::fs::File;
use std::io::Write;
use std::str::from_utf8;
use serde::{Serialize, Deserialize};
use bincode;
use crc16::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Packet {
    pub header: u64,
    pub sno: u32,
    pub payload_length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

fn calculate_checksum(data: &[u8]) -> u16 {
    State::<ARC>::calculate(data)
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let socket = UdpSocket::bind("192.168.190.217:12345").await?; // Client IP
    println!("UDP server waiting for packets...");

    let mut buf = [0u8; 1048];
    let mut file: Option<File> = None;

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        println!("📦 Received {} bytes from {}", len, addr); // ADD THIS
    
        let packet: Packet = match bincode::deserialize(&buf[..len]) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ Failed to deserialize: {}", e);
                continue;
            }
        };
    
        let calculated_checksum = calculate_checksum(&packet.payload);
        if packet.checksum != calculated_checksum {
            eprintln!("⚠ Checksum mismatch for packet {}", packet.sno);
            continue;
        }
    
        if packet.sno == 0 {
            let file_name = from_utf8(&packet.payload).unwrap();
            let sanitized_filename: String = file_name.chars().filter(|c| !"<>:\"/\\|?*".contains(*c)).collect();
            file = Some(File::create(&sanitized_filename).unwrap());
            println!("📂 Receiving file: {}", sanitized_filename);
        } else if let Some(f) = file.as_mut() {
            f.write_all(&packet.payload).expect("Failed to write data");
            println!("✍ Wrote packet {}", packet.sno);
        } else {
            println!("⚠ File not initialized. Skipping packet {}", packet.sno);
        }
    }
}
