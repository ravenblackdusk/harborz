mod choose_file;

use gtk::{Application, ApplicationWindow, Button, Grid};
use gtk::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt};
use gtk::traits::{ButtonExt, GridExt};
use crate::choose_file::CHOOSE_FILE;

fn main() {
    let application = Application::builder().application_id("com.kian.KD").build();
    application.connect_activate(|app| {
        let grid = Grid::builder().build();
        let button = Button::builder().label("Click me!").build();
        button.connect_clicked(CHOOSE_FILE);
        grid.attach(&button, 0, 0, 1, 1);
        grid.attach(&Button::builder().label("Click me!").build(), 1, 0, 2, 1);
        ApplicationWindow::builder().application(app).title("First GTK Program").child(&grid)
            .build().present();
    });
    application.run();
}
