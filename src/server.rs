use std::process::Command;
use get_if_addrs::get_if_addrs;
use std::net::IpAddr;

fn get_ipv4_from_interface(name: &str) -> Option<String> {
    for iface in get_if_addrs().ok()? {
        if iface.name == name {
            if let IpAddr::V4(ipv4) = iface.ip() {
                return Some(ipv4.to_string());
            }
        }
    }
    None
}

fn main() {
    let output = Command::new("ipconfig")
        .output()
        .expect("Failed to run ipconfig");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut interface_name:&str;
    for line in stdout.lines(){
        if line.contains("WiFi") || line.contains("Wi-Fi"){
            interface_name = line;
            print!("{}", get_ipv4_from_interface(interface_name).unwrap());
        }
    }

}
