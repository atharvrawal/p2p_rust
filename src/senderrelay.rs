use futures_util::{SinkExt, StreamExt};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Debug)]
struct RegisterResponse {
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerListResponse(Vec<String>);

#[derive(Serialize, Deserialize, Debug)]
struct RelayStatusResponse {
    status: String,
    target: Option<String>,
    initiator: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the signaling server
    let server_url = "ws://54.66.23.75:9876"; // Replace with your server address
    let url = Url::parse(server_url)?;
    let (mut ws_stream, _) = connect_async(url).await?;
    // println!("🔌 Connected to signaling server");

    // // Get a username from the user
    // println!("Enter your username:");
    // let mut username = String::new();
    // std::io::stdin().read_line(&mut username)?;
    // let username = username.trim().to_string();

    // // 1. Register with the server
    // let register_payload = json!({
    //     "type": "register",
    //     "username": username,
    //     "ip": "49.207.49.159",       // Placeholder - not used in relay mode
    //     "pip": "192.168.0.105",      // Placeholder - not used in relay mode
    //     "port": "11130"              // Placeholder - not used in relay mode
    // })
    // .to_string();
    
    // ws_stream.send(Message::Text(register_payload)).await?;
    
    // // Wait for registration confirmation
    // if let Some(msg) = ws_stream.next().await {
    //     if let Ok(Message::Text(response)) = msg {
    //         match serde_json::from_str::<RegisterResponse>(&response) {
    //             Ok(reg_response) => {
    //                 println!("✅ Registration status: {}", reg_response.status);
    //             },
    //             Err(_) => {
    //                 if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response) {
    //                     eprintln!("❌ Registration error: {}", error_response.error);
    //                     return Ok(());
    //                 }
    //             }
    //         }
    //     }
    // }

    // // 2. Request available peers
    // let request_peers_payload = json!({
    //     "type": "request_peer"
    // })
    // .to_string();
    
    // ws_stream.send(Message::Text(request_peers_payload)).await?;
    
    // // Receive peer list
    // let mut peer_list = Vec::new();
    // if let Some(msg) = ws_stream.next().await {
    //     if let Ok(Message::Text(response)) = msg {
    //         peer_list = serde_json::from_str::<Vec<String>>(&response)?;
    //         println!("👥 Available peers:");
    //         for (i, peer) in peer_list.iter().enumerate() {
    //             println!("  {}. {}", i + 1, peer);
    //         }
    //     }
    // }
    
    // // 3. Ask the user for the recipient's username
    // println!("Enter the username of the recipient:");
    // let mut target_username = String::new();
    // std::io::stdin().read_line(&mut target_username)?;
    // let target_username = target_username.trim().to_string();

    // 4. Initiate the relay with the target user
    let initiate_relay_payload = json!({
        "type": "initiate_relay",
        "target": target_username,
    })
    .to_string();
    
    ws_stream.send(Message::Text(initiate_relay_payload)).await?;

    // Wait for relay confirmation
    let mut relay_started = false;
    if let Some(msg) = ws_stream.next().await {
        if let Ok(Message::Text(response)) = msg {
            match serde_json::from_str::<RelayStatusResponse>(&response) {
                Ok(relay_response) => {
                    println!("🔄 Relay initiation status: {}", relay_response.status);
                    if relay_response.status == "relay_initiated" {
                        relay_started = true;
                    }
                },
                Err(_) => {
                    if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response) {
                        eprintln!("❌ Relay error: {}", error_response.error);
                        return Ok(());
                    }
                }
            }
        }
    }

    if !relay_started {
        eprintln!("❌ Failed to initiate relay with {}", target_username);
        return Ok(());
    }

    println!("✅ Relay established with {}. Ready to send files.", target_username);

    // 5. Select and send files until user chooses to exit
    // loop {
    //     println!("\nOptions:");
    //     println!("1. Send a file");
    //     println!("2. Exit");
    //     println!("Choose an option (1-2):");
        
    //     let mut choice = String::new();
    //     std::io::stdin().read_line(&mut choice)?;
    //     let choice = choice.trim();
        
    //     match choice {
    //         "1" => {
                // Select file to send
    let file_path = FileDialog::new().pick_file();
    if let Some(path) = file_path {
        println!("📂 Selected file: {}", path.display());
        send_file_via_websocket(&mut ws_stream, &path).await?;
        println!("✅ File transfer completed.");
    } else {
        println!("No file selected.");
    }
    //         },
    //         "2" => {
    //             println!("Ending relay and exiting...");
    //             // Signal the end of the relay
    //             let end_relay_payload = json!({
    //                 "type": "relay_control",
    //                 "action": "end",
    //             })
    //             .to_string();
    //             ws_stream.send(Message::Text(end_relay_payload)).await?;
    //             break;
    //         },
    //         _ => println!("Invalid option. Please try again."),
    //     }
    // }

    // Close the WebSocket connection
    ws_stream.close(None).await?;
    println!("Connection closed. Goodbye!");
    
    Ok(())
}

async fn send_file_via_websocket(
    ws_stream: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin + Send),
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
    let file_size = file.metadata()?.len();

    // Send file metadata as a JSON message
    let metadata_payload = json!({
        "type": "file_metadata",
        "name": file_name,
        "size": file_size
    }).to_string();
    
    ws_stream.send(Message::Text(metadata_payload)).await?;
    println!("📝 Sent file metadata: {} ({} bytes)", file_name, file_size);

    // Send file data in chunks
    let mut buffer = vec![0u8; 16384]; // 16KB chunks
    let mut total_sent = 0;
    
    while let Ok(bytes_read) = file.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        
        ws_stream.send(Message::Binary(buffer[..bytes_read].to_vec())).await?;
        
        total_sent += bytes_read;
        let progress = (total_sent as f64 / file_size as f64) * 100.0;
        print!("\r📦 Sent {}/{} bytes ({:.1}%)", total_sent, file_size, progress);
        std::io::stdout().flush()?;
    }
    
    println!("\n✅ File transfer completed: {} bytes sent.", total_sent);
    
    // Send end-of-file marker
    let eof_payload = json!({
        "type": "file_end",
        "name": file_name
    }).to_string();
    
    ws_stream.send(Message::Text(eof_payload)).await?;
    println!("🏁 End-of-file marker sent for: {}", file_name);
    
    Ok(())
}