use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use crate::common::constant::NONE;

pub trait PathString {
    fn to_path(&self) -> &Path;
}

impl PathString for String {
    fn to_path(&self) -> &Path {
        Path::new(self.as_str())
    }
}

pub fn format(timestamp: u64) -> String {
    format_pad(timestamp, 1)
}

pub fn format_pad(timestamp: u64, width: usize) -> String {
    let seconds = Duration::from_nanos(timestamp).as_secs();
    format!("{:0width$}:{:02}", seconds / 60, seconds % 60, width = width)
}

pub fn or_none(string: &Option<String>) -> &str {
    string.as_deref().unwrap_or(NONE)
}

pub fn or_none_static(string: Option<Rc<String>>) -> Rc<String> {
    string.unwrap_or(Rc::new(String::from(NONE)))
}
