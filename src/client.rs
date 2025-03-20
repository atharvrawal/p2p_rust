use tokio::net::TcpStream; // TcpStream represents an open tcp connection
use tokio::io::{AsyncWriteExt, AsyncReadExt};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    stream.write_all(b"Hello from client").await?;

    let mut buffer = [0; 1024];
    let size = stream.read(&mut buffer).await?;
    println!("Server says: {}", String::from_utf8_lossy(&buffer[..size]));

    Ok(())
}