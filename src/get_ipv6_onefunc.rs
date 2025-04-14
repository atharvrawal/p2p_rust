use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;

pub async fn send_json(json_data: &str) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect("54.66.23.75:8765").await?;

    // Send JSON
    stream.write_all(json_data.as_bytes()).await?;
    stream.flush().await?;

    // Read response (up to 8KB, adjust if needed)
    let mut buffer = vec![0u8; 8192];
    let n = stream.read(&mut buffer).await?;

    let response = String::from_utf8(buffer[..n].to_vec())?;
    Ok(response)
}

#[tokio::main]
async fn main() {
    let input = r#"{"msg": "hello"}"#;
    match send_json(input).await {
        Ok(response) => println!("Got: {}", response),
        Err(e) => eprintln!("Error: {}", e),
    }
}

