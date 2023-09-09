use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;
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

pub fn or_none_arc(string: Option<Arc<String>>) -> Arc<String> {
    string.unwrap_or(Arc::new(String::from(NONE)))
}

pub trait Plural: Display {
    fn plural(&self, string: &str) -> String;
    fn number_plural(&self, string: &str) -> String {
        format!("{self} {}", self.plural(string))
    }
}

impl Plural for i64 {
    fn plural(&self, string: &str) -> String {
        (*self as usize).plural(string)
    }
}

impl Plural for usize {
    fn plural(&self, string: &str) -> String {
        format!("{string}{}", if *self == 1 { "" } else { "s" })
    }
}
