use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use rfd::FileDialog;
use futures_util::{SinkExt, StreamExt};
use std::fs::File;
use std::io::BufReader;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunks

#[derive(Deserialize)]
struct RegisterResponse {
    status: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RelayStatusResponse {
    status: String,
    target: Option<String>,
    initiator: Option<String>,
}


fn main() {}

pub async fn relay_send(ws_stream: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>, target:String) -> Result<(), Box<dyn std::error::Error>> {
    let mut guard = ws_stream.lock().await;
    let (mut write, mut read) = StreamExt::split(&mut *guard);
    // println!("🔌 Connected to signaling server");

    // // Register user
    // println!("Enter your username:");
    // let mut username = String::new();
    // io::stdin().read_line(&mut username)?;
    // let username = username.trim();

    // let register_msg = json!({
    //     "type": "register",
    //     "username": username,
    //     "pip": "relay",
    //     "ip": "relay",
    //     "port": "relay"
    // }).to_string();
    // write.send(Message::Text(register_msg)).await?;

    // // Handle registration response
    // if let Some(Ok(Message::Text(resp))) = read.next().await {
    //     if let Ok(reg) = serde_json::from_str::<RegisterResponse>(&resp) {
    //         println!("✅ Registered: {}", reg.status);
    //     } else if let Ok(err) = serde_json::from_str::<ErrorResponse>(&resp) {
    //         eprintln!("❌ Error: {}", err.error);
    //         return Ok(());
    //     }
    // }

    // // Get peer list
    // write.send(Message::Text(json!({ "type": "request_peer" }).to_string())).await?;
    // let peers: Vec<String> = match read.next().await {
    //     Some(Ok(Message::Text(text))) => serde_json::from_str(&text)?,
    //     _ => {
    //         eprintln!("❌ Failed to get peers");
    //         return Ok(());
    //     }
    // };

    // println!("🧑‍🤝‍🧑 Available users:");
    // peers.iter()
    //     .enumerate()
    //     .filter(|(_, user)| user != &username)
    //     .for_each(|(i, user)| println!("  [{}] {}", i, user));

    // // Select peer
    // println!("\nSelect user number to connect to:");
    // let mut choice = String::new();
    // io::stdin().read_line(&mut choice)?;
    // let peer = &peers[choice.trim().parse::<usize>()?];

    // Initiate relay
    write.send(Message::Text(json!({
        "type": "initiate_relay",
        "target": target
    }).to_string())).await?;

    // Wait for relay confirmation
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(resp) = serde_json::from_str::<RelayStatusResponse>(&text) {
                if resp.status == "relay_initiated" {
                    println!("🔁 Relay session started with {}", target);
                    break;
                }
            }
        }
    }

    // File transfer
    if let Some(path) = FileDialog::new().pick_file() {
        let file_name = path.file_name().unwrap().to_string_lossy();
        let file_size = fs::metadata(&path)?.len();

        // Send metadata
        write.send(Message::Text(json!({
            "type": "file_metadata",
            "name": file_name,
            "size": file_size
        }).to_string())).await?;

        // Stream file chunks
        println!("📤 Sending {} ({} bytes)...", file_name, file_size);
        let mut file = BufReader::new(File::open(&path)?);
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut total_sent = 0;

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 { break }

            if let Err(e) = write.send(Message::Binary(buffer[..n].to_vec())).await {
                eprintln!("❌ Send error: {}", e);
                break;
            }
            total_sent += n;

            // Progress reporting
            print!("\r🚀 Progress: {:.1}%", (total_sent as f64 / file_size as f64) * 100.0);
            io::stdout().flush()?;
            tokio::task::yield_now().await;
        }

        // Finalize transfer
        println!("\n✅ File sent successfully!");
        write.send(Message::Text(json!({ "type": "file_end" }).to_string())).await?;
    }

    // Close connection
    if let Err(e) = write.close().await {
        eprintln!("❌ Error closing connection: {}", e);
    }
    Ok(())
}