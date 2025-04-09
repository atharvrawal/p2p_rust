// udp_sender.rs
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any port
    let target = "49.207.49.159:5815"; // Replace with your friend's public IP

    let message = b"Hello from Rust UDP!";
    socket.send_to(message, target)?;

    println!("Sent message to {}", target);
    Ok(())
}
