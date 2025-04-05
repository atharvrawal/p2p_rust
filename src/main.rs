use std::fs::File;
use std::io::{Write,Read};
use std::path::Path;
use rfd::FileDialog;
use std::str::from_utf8;
use serde::{Serialize, Deserialize};
use bincode;


#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {         // u-> unsigned, no after that -> number of bits allocated to integer
    header: u64,            // 8-byte magic number or protocol version
    sno: u32,               // 4-byte integer, to keep track of order 
    payload_length: u16,    // 2-byte integer
    checksum: u16,          // used for data integrity, can be CRC16 or simple XOR checksum
    payload: Vec<u8>        // Dynamic array for containing file data, It is heap allocated
}

pub fn file_to_packets(file_path: &Path) -> Vec<Packet> { // file_path is a reference to the actual path, rust automatically dereferences the variable
    let mut file = File::open(file_path).expect("Failed to open file"); // hence we can just use that variable in open()
    let mut packets = Vec::new(); // dynamically sized array, allocated upon use
    let mut buffer = [0; 1024]; // Fixed-size array instead of Vec, for the time being to make packets faster
    let mut seq_num = 1;
    let file_name = file_path.file_name().unwrap().to_str().unwrap(); // making filetype from OsStr {what .file_name() reuturns} to &str
    let initial_packet = Packet{header:0x12345678ABCDEF00, 
                                        sno: 0, 
                                        payload_length: (file_name.len() as u16),
                                        checksum: 0, 
                                        payload: file_name.as_bytes().to_vec()};
    packets.push(initial_packet);

    while let Ok(bytes_read) = file.read(&mut buffer) {
        if bytes_read == 0 {
            packets.push(Packet {
                header: 0x12345678ABCDEF00,
                sno: seq_num,
                payload_length: 0,
                checksum: 0,
                payload: buffer[..bytes_read].to_vec(),
            });
            break;
        }

        packets.push(Packet {
            header: 0x12345678ABCDEF00,
            sno: seq_num,
            payload_length: bytes_read as u16,
            checksum: 0,
            payload: buffer[..bytes_read].to_vec(),
        });
        seq_num += 1;
    }

    packets // no need to use return, we can just type the variable that needs to be returned
}


fn packets_to_file(packets: Vec<Packet>) {
    let file_name = from_utf8(&packets[0].payload).unwrap();
    let mut file = File::create(file_name).expect("Failed to create file");
    let mut num = 0;
    for packet in &packets[1..] { // starting the for loop from the seconds packet, (first is just file_name);
        file.write_all(&packet.payload).expect("Failed to write data");
        num += 1;
    }
    print!("{}",num);
}


fn main(){
    if let Some(path) = FileDialog::new().pick_file(){  
        let packets = file_to_packets(&path); // Path is a struct, hence we give the function pointer to path
        packets_to_file(packets); // to make sure variable remains of static size
    }
}





// Rust doesnt like having any room for errors, even possible errors
// this function doesnt work without handling the PlatformError
// because what if the slint file doesnt exist, it will not complie
// unless it is sure during static analysis that no runtime errors will occur

// runtime errors still may occus, but these are the ones that static analysis cant catch