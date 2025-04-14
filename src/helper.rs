use rand::RngCore;
use serde_json::{json, Value};
use slint::SharedString;
use std::net::{ToSocketAddrs, UdpSocket, IpAddr};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use stunclient::StunClient;
use tokio::runtime::Runtime;
use local_ip_address::list_afinet_netifas;
use get_if_addrs::{get_if_addrs, IfAddr};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::error::Error;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::MaybeTlsStream;


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

fn get_wifi_ip() -> String {
    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if iface.name.contains("Wi-Fi") || iface.name.contains("wlan") {
                if let IfAddr::V4(ipv4) = iface.addr {
                    return ipv4.ip.to_string();
                }
            }
        }
    }
    "Wi-Fi IP not found".to_string()
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
    let local_ip = get_wifi_ip();

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

pub fn print_json(value: &Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                println!("Key: {}", k);
                print_json(v);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                print_json(v);
            }
        }
        _ => {
            println!("Value: {}", value);
        }
    }
}

pub fn get_pip_port_json(username:&str, password:&str) -> serde_json::Value {
    let mut ipv4_ip = None;let mut ipv4_port = None;let mut ipv6_ip = None;let mut ipv6_port = None;
    let stun_hostname = "stun.l.google.com:19302"; 

    if let Some(stun_ipv4) = stun_hostname.to_socket_addrs().ok().unwrap().find(|a| a.is_ipv4()) {
        if let Ok(socket_v4) = UdpSocket::bind("0.0.0.0:42069") {
            let client_v4 = StunClient::new(stun_ipv4);
            if let Ok(addr) = client_v4.query_external_address(&socket_v4) {
                ipv4_ip = Some(addr.ip().to_string());
                ipv4_port = Some(addr.port());
            }
        }
    }

    if let Some(stun_ipv6) = stun_hostname.to_socket_addrs().ok().unwrap().find(|a| a.is_ipv6()) {
        if let Ok(socket_v6) = UdpSocket::bind("[::]:42070") {
            let client_v6 = StunClient::new(stun_ipv6);
            if let Ok(addr) = client_v6.query_external_address(&socket_v6) {
                ipv6_ip = Some(addr.ip().to_string());
                ipv6_port = Some(addr.port());
            }
        }
    }
    json!({"type" : "register","username":username,"password": password,"ipv4_ip": ipv4_ip,"ipv4_port": ipv4_port, "ipv6_ip": ipv6_ip,"ipv6_port": ipv6_port})
}


pub async fn send_json_value(json_value: &Value, ws_stream:Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>) -> tokio::io::Result<()>{
    let json_str = serde_json::to_string(json_value)?;
    let mut ws_stream = ws_stream.lock().await;
    ws_stream.send(Message::Text(json_str)).await.unwrap();
    Ok(())
}

pub async fn get_clients(ws_stream:Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>) -> Result<String, Box<dyn Error>> {
    let mut ws_stream = ws_stream.lock().await;
    let json_val = json!({"type":"getusers"});
    let json_str = serde_json::to_string(&json_val).unwrap();
    ws_stream.send(Message::Text(json_str)).await?;

    if let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => Ok(text),
            Message::Binary(bin) => Ok(String::from_utf8(bin)?),
            _ => Err("Unexpected message type".into()),
        }
    } else {
        Err("No response from server".into())
    }
}


// pub async fn get_users(){
//     let url = Url::parse("ws://54.66.23.75:9876").unwrap();
//     let json_str = r#"{"type": "get_users"}"#.to_string();
//     let (mut ws_stream, _) = connect_async(url).await.unwrap();
//     ws_stream.send(Message::Text(json_str)).await.unwrap();

// }

// pub async fn send_json(json_data: &str) -> Result<String, Box<dyn Error>> {
//     let mut stream = TcpStream::connect("54.66.23.75:8765").await?;

//     // Send JSON
//     stream.write_all((json_data.to_string() + "\n").as_bytes()).await?;
//     stream.flush().await?;

//     // Read response (up to 8KB, adjust if needed)
//     let mut buffer = vec![0u8; 8192];
//     let n = stream.read(&mut buffer).await?;

//     let response = String::from_utf8(buffer[..n].to_vec())?;
//     Ok(response)
// }

pub fn keys_from_json_str(json_str: String) -> Vec<String>{
    let _json: Value = serde_json::from_str(json_str.as_str()).unwrap();
    print!("{}",json_str);
    let mut usernames: Vec<String> = Vec::new();
    if let Value::Object(map) = _json {
        for (key, value) in map.iter() {
            usernames.push(key.clone());
        }
    }
    usernames
}