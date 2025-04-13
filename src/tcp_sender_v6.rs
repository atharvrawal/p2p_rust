use std::io::Write;
use std::net::TcpStream;

fn main() {
    let target = "[2401:4900:8838:54da:5d7e:74b8:4a01:e4c2]:42070";

    match TcpStream::connect(target) {
        Ok(mut stream) => {
            stream.write_all(b"hello").expect("Failed to send data");
            println!("✅ Sent 'hello' to {}", target);
        }
        Err(e) => {
            eprintln!("❌ Could not connect: {}", e);
        }
    }
}
