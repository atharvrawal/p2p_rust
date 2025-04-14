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
mod testtt;
use testtt::relay_send;

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
    let app = TestWindow::new().unwrap(); let app_weak = app.as_weak();
    
    let weak_app_register = app.as_weak();
    app.on_register(move |username: SharedString, password: SharedString| {
        let app_weak = weak_app_register.clone();
        task::block_in_place(move || {
            let pip_port_json = get_pip_port_json(username.as_str(), password.as_str());
            let pip_port_string = serde_json::to_string(&pip_port_json).unwrap();
            let _ = tokio::runtime::Runtime::new().unwrap().block_on(async {send_json_value(&pip_port_json).await});
            if let Some(app_strong) = app_weak.upgrade() {
                app_strong.set_output(SharedString::from(pip_port_string));
            }
        });
    });

    let weak_app_clients = app.as_weak();
    app.on_get_clients(move || {
        let app_weak = weak_app_clients.clone(); 
        slint::spawn_local(async move {
            let response = get_clients().await;
            let clients = keys_from_json_str(response.unwrap());
            println!("{:?}", clients);
            let model = ModelRc::new(VecModel::from(clients.into_iter().map(Into::into).collect::<Vec<_>>(),));
            app_weak.upgrade().unwrap().set_available_clients(model);
        }).unwrap();
    });

    let weak_app_target = app.as_weak();
    app.on_send(
        move |username: SharedString, target_username: SharedString| {
            let app_weak = weak_app_target.clone();
            let username = username.to_string();
            slint::spawn_local(async move {
                relay_send(target_username, username).await;
            }).unwrap();
    });
    

    app.on_file_picker(|| {
        if let Some(path) = FileDialog::new().pick_file() {
            print!("{}",path.to_str().unwrap());
            io::stdout().flush().unwrap();  // this forces the print to implement at the instant, instead of after the app closes
            path.to_str().unwrap().into() 
            
        } else {
            "".into() // return empty string if user cancels
        }
    });

    app.run().unwrap();
}