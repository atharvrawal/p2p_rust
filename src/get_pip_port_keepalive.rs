use std::net::{ToSocketAddrs, UdpSocket, SocketAddr};
use std::{thread, time::Duration};
use stunclient::StunClient;

fn main() {
    let stun_hostname = "stun.l.google.com:19302";
    println!("Resolving STUN server: {}", stun_hostname);

    let stun_server: SocketAddr = match stun_hostname.to_socket_addrs() {
        Ok(addrs) => {
            match addrs.into_iter().find(|a| a.is_ipv4()) {
                Some(ipv4_addr) => {
                    println!("Resolved STUN server to IPv4: {}", ipv4_addr);
                    ipv4_addr
                }
                None => {
                    eprintln!("No IPv4 address found for STUN server.");
                    return;
                }
            }
        }
        Err(err) => {
            eprintln!("DNS resolution failed: {}", err);
            return;
        }
    };

    let bind_addr = "0.0.0.0:42069";
    println!("Binding local UDP socket to {}", bind_addr);
    let socket = match UdpSocket::bind(bind_addr) {
        Ok(sock) => sock,
        Err(err) => {
            eprintln!("Failed to bind UDP socket: {}", err);
            return;
        }
    };

    let client = StunClient::new(stun_server);

    // Initial STUN request
    println!("Sending initial STUN request...");
    match client.query_external_address(&socket) {
        Ok(external_addr) => {
            println!("✅ Public IP: {}", external_addr.ip());
            println!("✅ Public Port: {}", external_addr.port());
        }
        Err(err) => {
            eprintln!("❌ Failed to get public IP: {}", err);
        }
    }

    // NAT Keep-Alive Loop
    println!("Starting NAT keep-alive every 25 seconds...");
    loop {
        match socket.send_to(b"keep-alive", stun_server) {
            Ok(_) => println!("🔁 Sent keep-alive to {}", stun_server),
            Err(err) => eprintln!("❌ Failed to send keep-alive: {}", err),
        }

        thread::sleep(Duration::from_secs(25));
    }
}
