mod choose_file;

use gtk::{Application, ApplicationWindow, Button, Grid};
use gtk::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt};
use gtk::traits::{ButtonExt, GridExt};
use crate::choose_file::choose;

fn main() {
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|app| {
        let grid = Grid::builder().build();
        let browse_button = Button::builder().label("browse").build();
        browse_button.connect_clicked(&choose);
        grid.attach(&browse_button, 0, 0, 1, 1);
        grid.attach(&Button::builder().label("Click me!").build(), 1, 0, 2, 1);
        ApplicationWindow::builder().application(app).title("music player").child(&grid).build()
            .present();
    });
    application.run();
}
