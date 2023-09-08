use std::cell::Cell;
use std::rc::Rc;
use adw::gdk::Rectangle;
use adw::prelude::*;
use gtk::GestureSwipe;

pub enum Direction {
    Horizontal,
    Vertical,
}

pub trait DirectionSwipe {
    fn connect_direction_swipe<F: Fn(&GestureSwipe, f64, f64, Box<dyn Fn(Direction) -> bool>) + 'static>(&self,
        do_on_swipe: F);
}

impl DirectionSwipe for GestureSwipe {
    fn connect_direction_swipe<F: Fn(&GestureSwipe, f64, f64, Box<dyn Fn(Direction) -> bool>) + 'static>(&self,
        do_on_swipe: F) {
        let begin_swipe_position = Rc::new(Cell::new(None::<(f64, f64)>));
        self.connect_begin({
            let begin_swipe_position = begin_swipe_position.clone();
            move |gesture, _| { begin_swipe_position.set(gesture.bounding_box_center()); }
        });
        let last_swipe_position = Rc::new(Cell::new(None::<(f64, f64)>));
        self.connect_update({
            let last_swipe_position = last_swipe_position.clone();
            move |gesture, _| { last_swipe_position.set(gesture.bounding_box_center()); }
        });
        self.connect_swipe(move |gesture, velocity_x, velocity_y| {
            if let Some((last_x, last_y)) = last_swipe_position.get() {
                let (begin_x, begin_y) = begin_swipe_position.get().unwrap();
                let bounding_box = Rectangle::new(last_x.min(begin_x) as i32, last_y.min(begin_y) as i32,
                    (last_x - begin_x).abs() as i32, (last_y - begin_y).abs() as i32);
                do_on_swipe(gesture, velocity_x, velocity_y, Box::new(move |direction| {
                    let (main_direction, other_direction) = match direction {
                        Direction::Horizontal => { (bounding_box.width(), bounding_box.height()) }
                        Direction::Vertical => { (bounding_box.height(), bounding_box.width()) }
                    };
                    0.1 * main_direction as f64 > other_direction as f64 && main_direction > 100
                }));
            }
        });
    }
}
