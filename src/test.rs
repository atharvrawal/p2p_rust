use eframe::App;
use egui::CentralPanel;
struct MyApp;

fn hello_world(){
    print!("hello world");
}


impl eframe::App for MyApp { // impl is used to bind functions to the struct MyApp, like having a function as a value
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
    }
}


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "P2P File Sharing in Rust",
        options, // it is a struct controlling the behaviour of a native window, such as position and size of the window
        Box::new(|_cc| Ok(Box::new(MyApp))), // Fix: Wrap in `Ok`
    )
}
