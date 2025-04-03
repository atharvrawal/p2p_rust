slint::include_modules!();

fn main(){
    let app = MainWindow::new().unwrap();
    app.run().unwrap();
    print!("hello world");
}