use slint::{Timer, TimerMode};
use rfd::FileDialog;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::Duration;
slint::include_modules!();


fn main(){
    let app = MainWindow::new().unwrap();
    let app_weak = app.as_weak();
    

    // let timer = Rc::new(Timer::default());
    // let timer_clone = timer.clone();

    // timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
    //     if let Some(app) = app_weak.upgrade() {
    //         app.invoke_tick(); // 🔁 this runs every 16ms 
    //     }
    // });



    app.on_register(||{
        let response = reqwest::blocking::get("https://api.ipify.org").unwrap().text().unwrap();
        print!("{}",response);
        io::stdout().flush().unwrap();
        response.into()

    });

    let a = 1;

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