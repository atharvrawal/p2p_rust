use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use serde::{Serialize, Deserialize};
use bincode;
use rfd::FileDialog;

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    pub header: u64,
    pub sno: u32,
    pub payload_length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

pub async fn send_file(file_path: &Path, server_ip: &str) -> tokio::io::Result<()> {
    let mut stream = TcpStream::connect(server_ip).await?;
    let packets = file_to_packets(file_path);

    for packet in packets {
        let encoded: Vec<u8> = bincode::serialize(&packet).unwrap();
        stream.write_all(&(encoded.len() as u32).to_be_bytes()).await?; // Send packet size first
        stream.write_all(&encoded).await?; // Send packet data
    }

    println!("File sent successfully.");
    Ok(())
}

pub fn file_to_packets(file_path: &Path) -> Vec<Packet> {
    let mut file = File::open(file_path).expect("Failed to open file");
    let mut packets = Vec::new();
    let mut buffer = [0; 1024];
    let mut seq_num = 1;

    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let initial_packet = Packet {
        header: 0x12345678ABCDEF00,
        sno: 0,
        payload_length: file_name.len() as u16,
        checksum: 0,
        payload: file_name.as_bytes().to_vec(),
    };
    packets.push(initial_packet);

    while let Ok(bytes_read) = file.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        packets.push(Packet {
            header: 0x12345678ABCDEF00,
            sno: seq_num,
            payload_length: bytes_read as u16,
            checksum: 0, // Implement checksum if needed
            payload: buffer[..bytes_read].to_vec(),
        });
        seq_num += 1;
    }

    packets
}

#[tokio::main]
async fn main() {
    let file_path = FileDialog::new().pick_file();
    if let Some(path) = file_path {
        send_file(&path, "127.0.0.1:8080").await.unwrap();
    } else {
        println!("No file selected.");
    }
}
