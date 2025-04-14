use rand::RngCore;
use serde_json::{json, Value};
use std::net::{ToSocketAddrs, UdpSocket, IpAddr};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use tokio::runtime::Runtime;
use local_ip_address::list_afinet_netifas;


pub async fn get_target_info(username: String) -> Option<Value> {
    let json_value = json!({
        "type": "peer_information",
        "target": username
    });

    let url = Url::parse("ws://54.66.23.75:8765").expect("Invalid WS URL");
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("❌ Failed to connect to WebSocket server");

    let (mut write, mut read) = ws_stream.split();

    // Send JSON request
    write
        .send(Message::Text(json_value.to_string()))
        .await
        .expect("❌ Failed to send message");

    // Await response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        // Try to parse as JSON
        if let Ok(json_resp) = serde_json::from_str::<Value>(&text) {
            return Some(json_resp);
        } else {
            eprintln!("❌ Failed to parse response as JSON");
        }
    } else {
        eprintln!("❌ No valid response from server");
    }

    None
}



pub async fn get_client() -> Vec<String> {
    let url = "ws://54.66.23.75:8765";
    let (ws_stream, _) = match connect_async(url).await {
        Ok(v) => v,
        Err(_) => return vec!["Failed to connect".into()],
    };

    let (mut write, mut read) = ws_stream.split();

    let request = json!({ "type": "request_peer" });
    if write.send(Message::Text(request.to_string())).await.is_err() {
        return vec!["Failed to send".into()];
    }

    if let Some(Ok(Message::Text(text))) = read.next().await {
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(val) => val
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect(),
            Err(_) => vec!["Invalid JSON".into()],
        }
    } else {
        vec!["No response".into()]
    }
}



pub fn get_pip_from_json(data: &Value) -> Option<String> {
    data.get("pip")?.as_str().map(|s| s.to_string())
}

fn get_local_ip() -> String {
    match local_ip_address::local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => "Could not determine private IP address".to_string(),
    }
}

pub fn get_pipp(username: String) -> Value {
    // Discover STUN server
    let stun_addr = ("stun.l.google.com", 19302)
        .to_socket_addrs()
        .unwrap()
        .find(|a| a.is_ipv4())
        .expect("Could not resolve STUN server");

    // Bind socket and discover private IP
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket");
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();

    // Get the private IP by connecting to stun and checking local_addr
    socket.connect(stun_addr).expect("Failed to connect to STUN server");
    let local_ip = get_local_ip();

    // STUN binding request
    let mut transaction_id = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut transaction_id);

    let mut request = Vec::new();
    request.extend_from_slice(&[0x00, 0x01]); // Binding Request
    request.extend_from_slice(&[0x00, 0x00]); // Message Length = 0
    request.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]); // Magic Cookie
    request.extend_from_slice(&transaction_id); // Transaction ID

    socket.send(&request).expect("Failed to send");

    let mut buf = [0u8; 1024];
    let len = socket.recv(&mut buf).expect("Failed to receive");

    // Parse STUN response
    let mut i = 20;
    while i + 4 < len {
        let attr_type = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let attr_len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;

        if attr_type == 0x0020 && attr_len >= 8 {
            let port = u16::from_be_bytes([buf[i + 6], buf[i + 7]]) ^ 0x2112;
            let ip_bytes = &buf[i + 8..i + 12];
            let ip = [
                ip_bytes[0] ^ 0x21,
                ip_bytes[1] ^ 0x12,
                ip_bytes[2] ^ 0xA4,
                ip_bytes[3] ^ 0x42,
            ];
            let ip_string = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);

            return json!({
                "type": "register",
                "username": username,
                "ip": ip_string,
                "port": port,
                "pip": local_ip
            });
        }

        i += 4 + attr_len;
        if attr_len % 4 != 0 {
            i += 4 - (attr_len % 4);
        }
    }

    json!({
        "error": "XOR-MAPPED-ADDRESS not found.",
        "pip": local_ip
    })
}
pub async fn send_register_payload(data: Value) {
    let url = Url::parse("ws://54.66.23.75:8765").expect("Invalid WS URL");
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("❌ Failed to connect to WebSocket server");

    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(data.to_string()))
        .await
        .expect("❌ Failed to send message");

    if let Some(Ok(reply)) = read.next().await {
        println!("🔁 Server replied: {}", reply);
    }
}

#[tokio::main]
async fn main() {
}