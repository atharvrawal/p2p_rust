use slint::{Timer, TimerMode, ModelRc, VecModel, SharedString, spawn_local};
use rfd::FileDialog;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::Duration;
slint::include_modules!();
use tokio_tungstenite::connect_async;
use tungstenite::Message;
use serde_json::json;
use futures_util::{StreamExt, SinkExt};
use tokio::runtime::Runtime;
use serde_json::{Value};
mod test2;
mod test;
use test2::get_pipp;
use test2::send_register_payload;
use test2::get_client;
use test::send_function;
use test2::get_pip_from_json;


fn print_json(value: &Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                println!("Key: {}", k);
                print_json(v);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                print_json(v);
            }
        }
        _ => {
            println!("Value: {}", value);
        }
    }
}

#[tokio::main]
async fn main(){
    let app = MainWindow::new().unwrap();
    let app_weak = app.as_weak();
    

    // let timer = Rc::new(Timer::default());
    // let timer_clone = timer.clone();

    // timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
    //     if let Some(app) = app_weak.upgrade() {
    //         app.invoke_tick(); // 🔁 this runs every 16ms 
    //     }
    // });



    app.on_register(|username: SharedString|{
        slint::spawn_local(async move {
            let response = get_pipp(username.to_string());
            send_register_payload(response).await;
            io::stdout().flush().unwrap();
        }).unwrap();
    });

    let weak_app_clients = app.as_weak();
    app.on_get_clients(move || {
        let app_weak = weak_app_clients.clone(); // inner clone for each call
        slint::spawn_local(async move {
            let usernames = get_client().await;
            let model = ModelRc::new(VecModel::from(usernames.into_iter().map(Into::into).collect::<Vec<_>>(),));
            app_weak.upgrade().unwrap().set_available_clients(model);
        }).unwrap();
    });

    let weak_app_target = app.as_weak();
    app.on_send(
        move |username: SharedString| {
            let app_weak = weak_app_target.clone();
            let username = username.to_string();
            slint::spawn_local(async move {
                let response = send(username).await.unwrap();
                let pip = get_pip_from_json(response).unwrap();
                send_function(pip);
                io::stdout().flush().unwrap();
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