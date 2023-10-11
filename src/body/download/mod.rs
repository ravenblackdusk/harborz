use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::mpsc::TryRecvError::{Disconnected, Empty};
use std::time::Duration;
use adw::gdk::gdk_pixbuf::Pixbuf;
use adw::glib::{timeout_add_local, timeout_add_local_once, Variant};
use adw::glib::ControlFlow::{Break, Continue};
use adw::prelude::*;
use adw::Window;
use bytes::{Buf, Bytes};
use gtk::{Adjustment, Button, Image, MenuButton, Overlay};
use log::{error, warn};
use once_cell::sync::Lazy;
use metal_archives::MetalArchives;
use crate::common::check_button_dialog::check_button_dialog;
use crate::common::constant::SUGGESTED_ACTION;

pub mod albums;
pub mod songs;

static METAL_ARCHIVES: Lazy<MetalArchives> = Lazy::new(|| { MetalArchives::new() });

fn append_download_button<DR: 'static, D: Fn(Sender<DR>) + 'static, S,
    HD: Fn(DR, Box<dyn Fn(anyhow::Result<Vec<anyhow::Result<S>>>)>,
        Box<dyn Fn(usize, anyhow::Result<Bytes>, Box<dyn Fn(&gtk::Box, &Image)>, usize)>) + Clone + 'static,
    HS: Fn(S) -> gtk::Box + Clone + 'static, CO: Fn(&Vec<Rc<RefCell<Vec<Option<Bytes>>>>>, usize) + Clone + 'static
>(download_label: &'static str, gtk_box: &gtk::Box, download: D, image_vec_count: usize, handle_download: HD,
    handle_search: HS, choose_option: CO, menu_button: MenuButton) {
    let download_button = Button::builder().label(format!("Download {download_label}")).build();
    gtk_box.append(&download_button);
    download_button.connect_clicked(move |_| {
        let (sender, receiver) = channel::<DR>();
        download(sender);
        let entities = Rc::new(RefCell::new(Vec::new()));
        let check_buttons = Rc::new(RefCell::new(Vec::new()));
        let images_vec = (0..image_vec_count).map(|_| { Rc::new(RefCell::new(Vec::new())) }).collect::<Vec<_>>();
        timeout_add_local(Duration::from_millis(500), {
            let handle_download = handle_download.clone();
            let handle_search = handle_search.clone();
            let choose_option = choose_option.clone();
            move || {
                loop {
                    match receiver.try_recv() {
                        Err(Empty) => { return Continue; }
                        Err(Disconnected) => { return Break; }
                        Ok(search_result) => {
                            handle_download(search_result, Box::new({
                                let entities = entities.clone();
                                let handle_search = handle_search.clone();
                                let check_buttons = check_buttons.clone();
                                let choose_option = choose_option.clone();
                                let images_vec = images_vec.clone();
                                move |search_result| {
                                    match search_result {
                                        Err(error) => {
                                            warn!("error searching to download [{download_label}] [{error}]");
                                        }
                                        Ok(search_vec) => {
                                            if search_vec.is_empty() {
                                                warn!("no [{download_label}] found for download");
                                            } else {
                                                entities.borrow_mut().extend(search_vec.into_iter()
                                                    .filter_map(|search| {
                                                        match search {
                                                            Ok(search) => Some(search),
                                                            Err(error) => {
                                                                error!("error trying to search to download \
                                                                [{download_label}] [{error}]");
                                                                None
                                                            }
                                                        }
                                                    }).map({
                                                    let handle_search = handle_search.clone();
                                                    move |it| { (handle_search(it), None::<Rc<i32>>) }
                                                }));
                                                check_buttons.borrow_mut().extend(check_button_dialog(
                                                    &format!("Choose the correct {download_label}"), None,
                                                    &entities.borrow(), "Choose", -1, SUGGESTED_ACTION,
                                                    RefCell::new({
                                                        let choose_option = choose_option.clone();
                                                        let images_vec = images_vec.clone();
                                                        move |_: &Overlay, variant: Variant, dialog: &Window| {
                                                            choose_option(&images_vec,
                                                                variant.get::<i32>().unwrap() as usize);
                                                            dialog.close();
                                                        }
                                                    }),
                                                ));
                                                for images in &images_vec {
                                                    images.borrow_mut().resize(entities.borrow().len(), None);
                                                }
                                            }
                                        }
                                    }
                                }
                            }), Box::new({
                                let entities = entities.clone();
                                let images_vec = images_vec.clone();
                                let check_buttons = check_buttons.clone();
                                move |i, bytes, handle_image, image_index| {
                                    match bytes {
                                        Ok(bytes) => {
                                            let (gtk_box, _) = &entities.borrow()[i];
                                            let image = Image::builder().pixel_size(92).build();
                                            handle_image(gtk_box, &image);
                                            *images_vec[image_index].borrow_mut().get_mut(i).unwrap()
                                                = Some(bytes.clone());
                                            image.set_from_pixbuf(Some(&Pixbuf::from_read(bytes.reader()).unwrap()));
                                            check_buttons.borrow()[i].set_action_target(Some((i as i32).to_variant()));
                                        }
                                        Err(error) => {
                                            warn!("error trying to download [{download_label}] result #[{i}] \
                                            [{error}]");
                                        }
                                    }
                                }
                            }));
                        }
                    }
                }
            }
        });
        menu_button.popdown();
    });
}

fn save(path: impl AsRef<Path>, vec: Bytes) {
    match File::create(path) {
        Ok(mut file) => {
            if let Err(error) = io::copy(&mut vec.reader(), &mut file) {
                error!("error saving image [{error}]");
            }
        }
        Err(error) => { error!("error creating image [{error}]"); }
    }
}

fn handle_scroll(scroll_adjustment: Option<f64>, adjustment: Adjustment) {
    if let Some(scroll_adjustment) = scroll_adjustment {
        timeout_add_local_once(Duration::from_millis(150), move || { adjustment.set_value(scroll_adjustment); });
    }
}
