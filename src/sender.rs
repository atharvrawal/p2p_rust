use tokio_tungstenite::connect_async;
use futures::SinkExt;
use std::fs;
use rfd::FileDialog;

#[tokio::main]
async fn main() {
    println!("Opening file picker...");
    let file_path = FileDialog::new()
        .set_title("Select a file to send")
        .pick_file();

    let Some(path) = file_path else {
        println!("No file selected.");
        return;
    };

    println!("Selected file: {}", path.display());

    let file_data = fs::read(&path).expect("Failed to read file");
    let url = url::Url::parse("ws://54.66.23.75:8765").unwrap();

    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    println!("Connected to receiver. Sending file...");

    let chunk_size = 1024;
    for chunk in file_data.chunks(chunk_size) {
        ws_stream.send(tokio_tungstenite::tungstenite::Message::Binary(chunk.to_vec()))
            .await
            .expect("Failed to send chunk");
    }

    println!("File sent!");
}
