use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Write, BufWriter};
use std::path::Path;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use rand::RngCore;
use slint::SharedString;
use std::net::{ToSocketAddrs, UdpSocket, IpAddr};
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

pub async fn relay_receive(username:String, ws_stream:Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut guard = ws_stream.lock().await;
    let (mut write, mut read) = StreamExt::split(&mut *guard);

        println!("🔌 Connected to signaling server");

    write.send(Message::Text(json!({
        "type": "register",
        "username": username.trim(),
        "pip": "relay",
        "ip": "relay",
        "port": "relay"
    }).to_string())).await?;

    // Prepare downloads directory
    let downloads = Path::new("downloads");
    if !downloads.exists() {
        fs::create_dir_all(downloads)?;
        println!("📁 Created downloads directory");
    }

    println!("📡 Waiting for files...");
    let mut current_file: Option<(String, File, u64)> = None;

    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    match data.get("type").and_then(|v| v.as_str()) {
                        Some("file_metadata") => {
                            if let (Some(name), Some(size)) = (
                                data.get("name").and_then(|v| v.as_str()),
                                data.get("size").and_then(|v| v.as_u64())
                            ) {
                                let path = downloads.join(name);
                                match File::create(&path) {
                                    Ok(file) => {
                                        println!("📥 Receiving {} ({} bytes)", name, size);
                                        current_file = Some((name.to_string(), file, size));
                                    },
                                    Err(e) => eprintln!("❌ File creation failed: {}", e),
                                }
                            }
                        },
                        Some("file_end") => {
                            if let Some((name, file, _)) = current_file.take() {
                                file.sync_all()?;
                                println!("\n✅ {} received successfully!", name);
                            }
                        },
                        Some("relay_initiated") => {
                            println!("🤝 Connected to {}", 
                                data.get("initiator").and_then(|v| v.as_str()).unwrap_or("unknown"));
                        },
                        _ => {}
                    }
                }
            },
            Message::Binary(data) => {
                if let Some((name, file, size)) = current_file.as_mut() {
                    if let Err(e) = file.write_all(&data) {
                        eprintln!("\n❌ Write failed: {}", e);
                        current_file = None;
                    } else {
                        // Progress reporting
                        let pos = file.metadata()?.len();
                        print!("\r📥 {}: {:.1}%", name, (pos as f64 / *size as f64) * 100.0);
                        io::stdout().flush()?;

                        // Periodic sync for large files
                        if pos % (5 * 1024 * 1024) == 0 {
                            file.sync_all()?;
                        }
                    }
                }
            },
            Message::Close(_) => break,
            _ => {}
        }
    }

    println!("👋 Session ended");
    Ok(())
}


#[tokio::main]
async fn main(){
    let (ws_stream, _) = connect_async("ws://54.66.23.75:8765").await.expect("Failed to connect");
    let ws_stream = Arc::new(Mutex::new(ws_stream));
    let ws_stream_clone_get_clients = ws_stream.clone();
    relay_receive("atharv".to_string(), ws_stream_clone_get_clients).await.unwrap();
}


