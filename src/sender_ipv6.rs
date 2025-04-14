// sender.rs
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use serde_json::{Value,json};

pub async fn send_json_value(json_value: &Value) -> tokio::io::Result<()> {
    let mut stream = TcpStream::connect("54.66.23.75:8765").await?;
    let mut json_str = serde_json::to_string(json_value)?;
    json_str.push('\n'); // Important: server expects newline   
    stream.write_all(json_str.as_bytes()).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let data = json!({
        "type": "register",
        "msg": "Hello from Rust!"
    });

    if let Err(e) = send_json_value(&data).await {
        eprintln!("Error sending JSON: {}", e);
    }
}