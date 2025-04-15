use helper::send_json_value;
use slint::{Timer, TimerMode, ModelRc, VecModel, SharedString, spawn_local};
use rfd::FileDialog;
use std::io::{self, Write};
slint::include_modules!();
use serde_json::{Value};    
use tokio::net::TcpStream;
use tokio::task;
mod helper;
use helper::print_json;
use helper::get_pip_port_json;
use helper::keys_from_json_str;
use helper::get_clients;
use std::error::Error;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
mod true_test;
use true_test::relay_send;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio::sync::Mutex;
use std::sync::Arc;
mod test_receiver;
use test_receiver::relay_receive;

fn process_input_json(input: SharedString) {
    let input_str = input.as_str();

    let json: Value = match serde_json::from_str(input_str) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            return;
        }
    };

    // Now you can access it like any other JSON
    println!("Parsed JSON: {:?}", json);
}

#[tokio::main]
async fn main(){
    let (ws_stream, _) = connect_async("ws://54.66.23.75:8765").await.expect("Failed to connect");
    let ws_stream = Arc::new(Mutex::new(ws_stream));
    let app = TestWindow::new().unwrap(); 
    let app_weak = app.as_weak();
    
    // Register event handler
    let weak_app_register = app.as_weak();
    let ws_stream_clone_register = ws_stream.clone();
    app.on_register(move |username: SharedString, password: SharedString| {
        let app_weak = weak_app_register.clone();
        let ws_stream = ws_stream_clone_register.clone();
        
        // Use async block directly
        let _ = slint::spawn_local(async move {
            let pip_port_json = get_pip_port_json(username.as_str(), password.as_str());
            let pip_port_string = serde_json::to_string(&pip_port_json).unwrap();
            
            // Send JSON asynchronously
            if let Err(e) = send_json_value(&pip_port_json, ws_stream).await {
                eprintln!("Error sending JSON: {}", e);
            }

            // Update UI output
            if let Some(app_strong) = app_weak.upgrade() {
                app_strong.set_output(SharedString::from(pip_port_string));
            }
        });
    });

    // Get clients event handler
    let weak_app_clients = app.as_weak();
    let ws_stream_clone_get_clients = ws_stream.clone();
    app.on_get_clients(move || {
        let app_weak = weak_app_clients.clone(); 
        let ws_stream = ws_stream_clone_get_clients.clone();
        slint::spawn_local(async move {
            let response = get_clients(ws_stream).await;
            let clients = keys_from_json_str(response.unwrap());
            println!("{:?}", clients);
            let model = ModelRc::new(VecModel::from(clients.into_iter().map(Into::into).collect::<Vec<_>>(),));
            app_weak.upgrade().unwrap().set_available_clients(model);
        }).unwrap();
    });

    let ws_stream_clone_send = ws_stream.clone();
    let weak_app_target = app.as_weak();
    app.on_send(
        move |target_username: SharedString| {
            let app_weak = weak_app_target.clone();
            let ws_stream = ws_stream_clone_send.clone();
            slint::spawn_local(async move {
                if let Err(e) = relay_receive(target_username.to_string(), ws_stream).await {
                    eprintln!("Error relaying: {}", e);
                }
            }).unwrap();
        }
    );

    let ws_stream_clone_receive = ws_stream.clone();
    let weak_app_target = app.as_weak();
    app.on_recieve(
        move |username: SharedString| {
            let app_weak = weak_app_target.clone();
            let ws_stream = ws_stream_clone_receive.clone();
            let username = username.to_string();
            slint::spawn_local(async move {
                // Call relay_send asynchronously
                if let Err(e) = relay_receive(username.to_string(), ws_stream).await {
                    eprintln!("Error relaying: {}", e);
                }
            }).unwrap();
        }
    );

    // File picker event handler
    app.on_file_picker(|| {
        if let Some(path) = FileDialog::new().pick_file() {
            print!("{}", path.to_str().unwrap());
            io::stdout().flush().unwrap(); // Forces print immediately
            path.to_str().unwrap().into() 
        } else {
            "".into() // return empty string if user cancels
        }
    });

    // Run the app
    app.run().unwrap();
}