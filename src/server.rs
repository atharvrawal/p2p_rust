// file: receiver.rs
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures::{StreamExt};
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() {
    let addr = "54.66.23.75:8765";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    println!("Receiver listening on {}", addr);

    let (stream, _) = listener.accept().await.expect("Failed to accept");
    let mut ws_stream = accept_async(stream).await.expect("Error during handshake");

    let mut file = File::create("received_file").expect("Could not create file");

    println!("Connection established. Receiving file...");

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.expect("Failed to receive message");
        if msg.is_binary() {
            file.write_all(&msg.into_data()).expect("Failed to write to file");
        }
    }

    println!("File received!");
}
