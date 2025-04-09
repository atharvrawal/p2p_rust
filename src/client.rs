use tokio::net::UdpSocket;
use std::fs::File;
use std::io::Write;
use std::str::from_utf8;
use serde::{Serialize, Deserialize};
use bincode;
use crc16::*;

#[derive(Serialize, Deserialize, Debug, Clone)]  // Added Clone
pub struct Packet {
    pub header: u64,
    pub sno: u32,
    pub payload_length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

#[tokio::main]  
async fn main() -> tokio::io::Result<()> {
    let socket = UdpSocket::bind("192.168.190.217:12345").await?;
    println!("UDP server waiting for packets...");

    let mut packets = Vec::new();
    let mut buf = [0u8; 1048];

    loop {
        let (len, _) = socket.recv_from(&mut buf).await?;
        let packet: Packet = bincode::deserialize(&buf[..len]).unwrap();

        let calculated_checksum = calculate_checksum(&packet.payload);
        if packet.checksum != calculated_checksum {
            eprintln!("Checksum mismatch for packet {}", packet.sno);
            continue;
        }

        packets.push(packet.clone());  // Clone works now!
        println!("Received packet {}", packet.sno);

        if packet.sno != 0 && packet.payload_length < 1024 {
            break;
        }
    }

    if !packets.is_empty() {
        packets_to_file(packets);
        println!("File saved successfully!");
    }
    Ok(())
}

fn packets_to_file(mut packets: Vec<Packet>) {
    let file_name = from_utf8(&packets[0].payload).unwrap();
    let sanitized_filename: String = file_name
    .chars()
    .filter(|c| !"<>:\"/\\|?*".contains(*c))
    .collect();

    let mut file = File::create(sanitized_filename).unwrap();
    println!("Total packets received: {}", packets.len());
    for p in &packets {
        println!("Packet {}: {} bytes", p.sno, p.payload_length);
    }
    packets[1..].sort_by(|a, b| a.sno.cmp(&b.sno));
    for packet in &packets[1..] {
        file.write_all(&packet.payload).expect("Failed to write data");
    }
}

fn calculate_checksum(data: &[u8]) -> u16 {
    State::<ARC>::calculate(data)
}