use gtk::{Application, ApplicationWindow, Button};
use gtk::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt};

fn main() {
    let application = Application::builder().application_id("com.kian.KD").build();
    application.connect_activate(|app| {
        ApplicationWindow::builder().application(app).title("First GTK Program")
            .child(&Button::builder().label("Click me!").build()).build().present();
    });
    application.run();
}
