use std::sync::mpsc::Sender;
use std::usize;
use bytes::Bytes;

pub struct ArtistSearch {
    pub name: String,
    pub genre: String,
    pub location: String,
}

pub enum DownloadArtistEvent {
    SearchResult(anyhow::Result<Vec<anyhow::Result<ArtistSearch>>>),
    Logo(usize, anyhow::Result<Bytes>),
    Photo(usize, anyhow::Result<Bytes>),
}

pub struct AlbumSearch {
    pub artist: String,
    pub album: String,
    pub album_type: String,
}

pub enum DownloadAlbumEvent {
    SearchResult(anyhow::Result<Vec<anyhow::Result<AlbumSearch>>>),
    Cover(usize, anyhow::Result<Bytes>),
}

pub trait MetadataFetcher {
    fn download_artist_logo_and_photo(&'static self, artist: &str, sender: Sender<DownloadArtistEvent>);
    fn download_cover(&'static self, artist: &str, album: &str, sender: Sender<DownloadAlbumEvent>);
}
