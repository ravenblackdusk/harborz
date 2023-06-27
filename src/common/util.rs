use std::path::Path;

pub trait PathString {
    fn to_path(&self) -> &Path;
}

impl PathString for String {
    fn to_path(&self) -> &Path {
        Path::new(self.as_str())
    }
}
