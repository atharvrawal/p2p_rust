use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use bincode;

pub fn packets_to_file(mut packets: Vec<Packet>) {
    let file_name = from_utf8(&packets[0].payload).unwrap();
    let mut file = File::create(file_name).expect("Failed to create file");
    println!("Total packets received: {}", packets.len());
    for p in &packets {
        println!("Packet {}: {} bytes", p.sno, p.payload_length);
    }
    packets[1..].sort_by(|a, b| a.sno.cmp(&b.sno));
    for packet in &packets[1..] {
        file.write_all(&packet.payload).expect("Failed to write data");
    }
}






#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080").await?;
    println!("UDP server waiting for packets...");
    let mut _s = 0;
    let mut packets = Vec::new();
    let mut buf = [0u8; 65535];

    loop {
        let (len, peer) = socket.recv_from(&mut buf).await?;
        let packet: Packet = bincode::deserialize(&buf[..len]).unwrap();

        let calculated_checksum = calculate_checksum(&packet.payload);
        if packet.checksum != calculated_checksum {
            eprintln!("❌ Checksum mismatch for packet {}", packet.sno);
            continue;
        }
        if packet.sno != _s {
            println!("⚠ Unexpected packet {} (expected {})", packet.sno, _s);
        
            // Send ACK again if this was a duplicate
            if packet.sno < _s {
                let ack_msg = format!("ACK:{}", packet.sno);
                socket.send_to(ack_msg.as_bytes(), peer).await?;
                println!("🔁 Re-ACKed packet {}", packet.sno);
            }
            continue;
        }        
        packets.push(packet.clone());
        _s += 1;
        println!("📥 Received packet {}", packet.sno);

        // Send ACK
        let ack_msg = format!("ACK:{}", packet.sno);
        socket.send_to(ack_msg.as_bytes(), peer).await?;

        if packet.sno != 0 && packet.payload_length < 1024 {
            break;
        }
    }

    if !packets.is_empty() {
        packets_to_file(packets);
        println!("📁 File saved successfully!");
    }
    Ok(())
}