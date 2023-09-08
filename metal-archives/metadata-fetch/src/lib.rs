use std::path::Path;

pub trait MetadataFetcher {
    fn download_artist_logo_and_photo(&self, artist: &str, path: impl AsRef<Path>) -> anyhow::Result<()>;
    fn download_cover(&self, artist: &str, album: &str, path: impl AsRef<Path>) -> anyhow::Result<()>;
}
