use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
struct RegisterResponse {
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RelayStatusResponse {
    status: String,
    target: Option<String>,
    initiator: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileMetadata {
    name: String,
    size: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the signaling server
    let server_url = "ws://54.66.23.75:9876"; // Replace with your server address
    let url = Url::parse(server_url)?;
    let (mut ws_stream, _) = connect_async(url).await?;
    println!("🔌 Connected to signaling server");

    // // Get username from the user
    // println!("Enter your username:");
    // let mut username = String::new();
    // std::io::stdin().read_line(&mut username)?;
    // let username = username.trim().to_string();

    // // 1. Register with the server
    // let register_payload = json!({
    //     "type": "register",
    //     "username": username,
    //     "ip": "127.0.0.1",       // Placeholder - not used in relay mode
    //     "pip": "127.0.0.1",      // Placeholder - not used in relay mode
    //     "port": "0"              // Placeholder - not used in relay mode
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

    // println!("📡 Waiting for incoming relay connections...");

    // Create downloads directory if it doesn't exist
    let send_payload = json!({
        "type" = "relay_receive"
        "username" = username;
    })
    .to_string();

    ws_stream.send(Message::Text(send_payload)).await?;
    let download_dir = Path::new("downloads");
    if !download_dir.exists() {
        fs::create_dir_all(download_dir)?;
        println!("📁 Created downloads directory");
    }

    // Process incoming messages
    let mut current_file: Option<(String, File, u64, u64)> = None; // (filename, file handle, size, bytes received)

    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let json_result: Result<Value, _> = serde_json::from_str(&text);
                
                if let Ok(json_data) = json_result {
                    match json_data.get("type").and_then(|v| v.as_str()) {
                        Some("relay_initiated") => {
                            if let Some(initiator) = json_data.get("initiator").and_then(|v| v.as_str()) {
                                println!("🤝 Relay initiated by: {}", initiator);
                                println!("📥 Ready to receive files...");
                            }
                        },
                        Some("file_metadata") => {
                            let file_name = json_data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown_file");
                            let file_size = json_data.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                            
                            // Create a file in the downloads directory
                            let file_path = download_dir.join(file_name);
                            match File::create(&file_path) {
                                Ok(file) => {
                                    println!("📄 Receiving file: {} ({} bytes)", file_name, file_size);
                                    current_file = Some((file_name.to_string(), file, file_size, 0));
                                },
                                Err(e) => {
                                    eprintln!("❌ Error creating file: {}", e);
                                    current_file = None;
                                }
                            }
                        },
                        Some("file_end") => {
                            if let Some((filename, _, total_size, received_size)) = &current_file {
                                println!("\n✅ File received: {} ({}/{} bytes)", filename, received_size, total_size);
                                current_file = None;
                            }
                        },
                        Some("relay_control") => {
                            if let Some(action) = json_data.get("action").and_then(|v| v.as_str()) {
                                if action == "end" {
                                    println!("🔚 Relay ended by the sender.");
                                    break;
                                }
                            }
                        },
                        Some("error") => {
                            if let Some(message) = json_data.get("error").and_then(|v| v.as_str()) {
                                eprintln!("❌ Error: {}", message);
                            }
                        },
                        Some(other) => {
                            println!("📩 Received message type: {}", other);
                        },
                        None => {
                            println!("📩 Received message without type field: {}", text);
                        }
                    }
                } else {
                    println!("📩 Received non-JSON text: {}", text);
                }
            },
            Ok(Message::Binary(data)) => {
                if let Some((filename, file, total_size, received_size)) = &mut current_file {
                    match file.write_all(&data) {
                        Ok(_) => {
                            // Update received size
                            *received_size += data.len() as u64;
                            
                            // Show progress
                            let progress = (*received_size as f64 / *total_size as f64) * 100.0;
                            print!("\r📥 Receiving {}: {}/{} bytes ({:.1}%)", 
                                   filename, received_size, total_size, progress);
                            io::stdout().flush()?;
                        },
                        Err(e) => {
                            eprintln!("\n❌ Error writing to file: {}", e);
                        }
                    }
                } else {
                    println!("📦 Received {} bytes of data without an active file", data.len());
                }
            },
            Ok(Message::Close(_)) => {
                println!("🔌 Connection closed by the server");
                break;
            },
            Ok(_) => {},
            Err(e) => {
                eprintln!("❌ WebSocket error: {}", e);
                break;
            }
        }
    }

    // Close any open file
    if current_file.is_some() {
        println!("\n Connection closed while receiving a file. The file may be incomplete.");
        current_file = None;
    }

    println!("Goodbye!");
    Ok(())
}