use tokio::net::UdpSocket;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use serde::{Serialize, Deserialize};
use bincode;
use rfd::FileDialog;
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
async fn main() {
    let file_path = FileDialog::new().pick_file();
    if let Some(path) = file_path {
        send_file_udp(&path, "192.168.190.217:12345").await.unwrap(); // Client IP
    } else {
        println!("No file selected.");
    }
}

pub async fn send_file_udp(file_path: &Path, server_addr: &str) -> tokio::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(server_addr).await?;

    let mut file = File::open(file_path).unwrap();
    let file_name = file_path.file_name().unwrap().to_str().unwrap();

    // Send filename as first packet
    let name_packet = Packet {
        header: 0x12345678ABCDEF00,
        sno: 0,
        payload_length: file_name.len() as u16,
        checksum: calculate_checksum(file_name.as_bytes()),
        payload: file_name.as_bytes().to_vec(),
    };
    socket.send(&bincode::serialize(&name_packet).unwrap()).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let mut buffer = [0u8; 1024];
    let mut seq_num = 1;

    loop {
        let bytes_read = file.read(&mut buffer).unwrap();
        if bytes_read == 0 { break; }

        let data = &buffer[..bytes_read];
        let packet = Packet {
            header: 0x12345678ABCDEF00,
            sno: seq_num,
            payload_length: bytes_read as u16,
            checksum: calculate_checksum(data),
            payload: data.to_vec(),
        };

        socket.send(&bincode::serialize(&packet).unwrap()).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        seq_num += 1;
    }

    println!("File sent via UDP.");
    Ok(())
}
