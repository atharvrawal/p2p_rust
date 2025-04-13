use std::net::{ToSocketAddrs, UdpSocket, SocketAddr};
use std::{thread, time::Duration};
use stunclient::StunClient;

fn resolve_stun_addr(stun_hostname: &str, ipv6: bool) -> Option<SocketAddr> {
    stun_hostname.to_socket_addrs().ok()?.find(|a| a.is_ipv6() == ipv6)
}

fn bind_socket(ipv6: bool, port: u16) -> Option<UdpSocket> {
    let bind_addr = if ipv6 { "[::]:".to_string() } else { "0.0.0.0:".to_string() };
    UdpSocket::bind(format!("{}{}", bind_addr, port)).ok()
}

fn query_and_print(client: &StunClient, socket: &UdpSocket, label: &str) {
    println!("Sending STUN request for {}...", label);
    match client.query_external_address(socket) {
        Ok(addr) => {
            println!("✅ {} IP: {}", label, addr.ip());
            println!("✅ {} Port: {}", label, addr.port());
        }
        Err(err) => eprintln!("❌ Failed to get {} IP: {}", label, err),
    }
}

fn main() {
    let stun_hostname = "stun.l.google.com:19302";

    // IPv4
    if let Some(stun_ipv4) = resolve_stun_addr(stun_hostname, false) {
        println!("Resolved IPv4 STUN server: {}", stun_ipv4);
        if let Some(socket_v4) = bind_socket(false, 42069) {
            let client_v4 = StunClient::new(stun_ipv4);
            query_and_print(&client_v4, &socket_v4, "IPv4");
        }
    } else {
        eprintln!("No IPv4 STUN address found.");
    }

    // IPv6
    if let Some(stun_ipv6) = resolve_stun_addr(stun_hostname, true) {
        println!("Resolved IPv6 STUN server: {}", stun_ipv6);
        if let Some(socket_v6) = bind_socket(true, 42070) {
            let client_v6 = StunClient::new(stun_ipv6);
            query_and_print(&client_v6, &socket_v6, "IPv6");
        }
    } else {
        eprintln!("No IPv6 STUN address found.");
    }

    // Optional: NAT Keep-alive (only for IPv4 here, add IPv6 if needed)
    // loop {
    //     socket_v4.send_to(b"keep-alive", stun_ipv4).ok();
    //     thread::sleep(Duration::from_secs(25));
    // }
}
