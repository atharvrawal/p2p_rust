use slint::{Timer, TimerMode, ModelRc, VecModel, SharedString, spawn_local};
use rfd::FileDialog;
use std::io::{self, Write};
slint::include_modules!();
use serde_json::{Value};    
mod helper;
use helper::print_json;

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
    
    let app = TestWindow::new().unwrap();
    let app_weak = app.as_weak();
    

    // let timer = Rc::new(Timer::default());
    // let timer_clone = timer.clone();

    // timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
    //     if let Some(app) = app_weak.upgrade() {
    //         app.invoke_tick(); // 🔁 this runs every 16ms 
    //     }
    // });


    let weak_app_register = app.as_weak();
    app.on_register(move |username: SharedString, password: SharedString| {
        let app_weak = weak_app_register.clone();
        let json_data: Value = serde_json::from_str(r#"{"name": "Slint", "version": 1}"#).unwrap();
        let full_json_string = serde_json::to_string(&json_data).unwrap();
        if let Some(app_strong) = app_weak.upgrade() {
            app_strong.set_output(SharedString::from(full_json_string));
        }
    });
    

    let weak_app_clients = app.as_weak();
    app.on_get_clients(move || {
        let app_weak = weak_app_clients.clone(); // inner clone for each call
    });

    let weak_app_target = app.as_weak();
    app.on_send(
        move |username: SharedString| {
            let app_weak = weak_app_target.clone();
            let username = username.to_string();
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