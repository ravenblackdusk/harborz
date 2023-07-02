use std::path::Path;
use std::time::Duration;

pub trait PathString {
    fn to_path(&self) -> &Path;
}

impl PathString for String {
    fn to_path(&self) -> &Path {
        Path::new(self.as_str())
    }
}

pub fn format(timestamp: u64) -> String {
    let seconds = Duration::from_nanos(timestamp).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
