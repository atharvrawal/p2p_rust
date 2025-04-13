use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("[::]:42070").expect("Failed to bind");
    println!("Listening on [::]:42070 (IPv6 UDP)");

    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                let msg = String::from_utf8_lossy(&buf[..len]);
                println!("Received from [{}]: {}", src, msg);
            }
            Err(e) => eprintln!("Receive failed: {}", e),
        }
    }
}
