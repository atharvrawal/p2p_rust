use std::net::UdpSocket;

fn main() {
    let target = "[2401:4900:8838:54da:5d7e:74b8:4a01:e4c2]:42070";

    // Bind to any local IPv6 address and OS-assigned port
    let socket = UdpSocket::bind("[::]:0").expect("Failed to bind");

    socket.send_to(b"hello", target).expect("Failed to send");
    println!("✅ Sent 'hello' to {}", target);
}
