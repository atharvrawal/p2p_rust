use tokio::net::{TcpListener, TcpStream}; // TcpListner listens for incoming connections
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // Provides async read/write function

async fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    if let Ok(size) = stream.read(&mut buffer).await {
        println!("Received: {}", String::from_utf8_lossy(&buffer[..size]));
        let _ = stream.write_all(b"Hello from server").await;
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Server listening on port 8080...");

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_client(socket).await;
        });
    }
}
