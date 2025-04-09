use tokio::net::UdpSocket;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use serde::{Serialize, Deserialize};
use bincode;
use rfd::FileDialog;
use crc16::*;
use tokio::time::{timeout, Duration};


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

// ... [imports and Packet struct unchanged]



pub async fn send_file_udp(file_path: &Path, server_addr: &str) -> tokio::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(server_addr).await?;

    let mut file = File::open(file_path).unwrap();
    let file_name = file_path.file_name().unwrap().to_str().unwrap();

    let mut packet_count = 0;

    // Send filename as first packet
    let name_packet = Packet {
        header: 0x12345678ABCDEF00,
        sno: 0,
        payload_length: file_name.len() as u16,
        checksum: calculate_checksum(file_name.as_bytes()),
        payload: file_name.as_bytes().to_vec(),
    };
    send_with_ack(&socket, &name_packet).await?;
    packet_count += 1;

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

        send_with_ack(&socket, &packet).await?;
        packet_count += 1;
        seq_num += 1;
    }

    println!("✅ File sent via UDP. Total packets sent: {}", packet_count);
    Ok(())
}

async fn send_with_ack(socket: &UdpSocket, packet: &Packet) -> tokio::io::Result<()> {
    let packet_bytes = bincode::serialize(&packet).unwrap();
    let expected_ack = format!("ACK:{}", packet.sno);
    let mut buf = [0u8; 128];

    loop {
        socket.send(&packet_bytes).await?;
        println!("📤 Sent packet {}", packet.sno);

        match timeout(Duration::from_millis(500), socket.recv(&mut buf)).await {
            Ok(Ok(received)) => {
                let ack_msg = String::from_utf8_lossy(&buf[..received]);
                if ack_msg == expected_ack {
                    println!("✅ Received ACK for {}", packet.sno);
                    break;
                }
            }
            _ => {
                println!("⏳ Timeout for packet {}, retrying...", packet.sno);
            }
        }
    }
    Ok(())
}

#[tokio::main]
pub async fn send_function(pip:String) {
    // Pick a file using the dialog
    let file_path = FileDialog::new()
        .set_title("Select file to send")
        .pick_file();

    match file_path {
        Some(path) => {
            let server_addr_string = format!("{}:8080", pip);
            let server_addr: &str = &server_addr_string;
            match send_file_udp(&path, server_addr).await {
                Ok(_) => println!("🎉 File transfer completed successfully."),
                Err(e) => eprintln!("❌ Error during file transfer: {}", e),
            }
        }
        None => {
            println!("⚠ No file selected.");
        }
    }
}

fn main(){}