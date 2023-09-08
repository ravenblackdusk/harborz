use std::fs::File;
use std::io;
use std::path::Path;
use anyhow::anyhow;
use libproxy::ProxyFactory;
use scraper::{Html, Selector};
use ureq::{Agent, AgentBuilder, Proxy};
use serde_derive::Deserialize;
use url::Url;
use metadata_fetch::MetadataFetcher;

pub struct MetalArchives {
    base_uri: Url,
    agent: Agent,
}

impl MetalArchives {
    pub fn new() -> Self {
        const BASE_URI: &'static str = "https://www.metal-archives.com";
        Self {
            base_uri: Url::parse(BASE_URI).unwrap(),
            agent: if let [proxy] = &ProxyFactory::new().unwrap().get_proxies(BASE_URI).unwrap()[..] {
                AgentBuilder::new().proxy(Proxy::new(&proxy).unwrap()).build()
            } else {
                Agent::new()
            },
        }
    }
    fn get_search_response(&self, uri: &Url) -> anyhow::Result<Vec<String>> {
        let mut search_response = self.agent.get(uri.as_str()).call()?.into_json::<SearchResponse>()?;
        if search_response.aa_data.len() == 1 {
            Ok(search_response.aa_data.drain(0..1).next().unwrap())
        } else {
            Err(anyhow!("{} matches found [{:?}]", search_response.aa_data.len(), search_response.aa_data))
        }
    }
    fn download(&self, uri_fragment: &str, ids: Vec<&str>, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let a = Html::parse_fragment(uri_fragment);
        let uri = a.select(&Selector::parse("a").unwrap()).next().unwrap().value().attr("href").unwrap();
        let response = self.agent.get(uri).call()?.into_string()?;
        let html = Html::parse_document(&response);
        let selector = Selector::parse(&ids.into_iter().map(|id| { format!("#{}", id) }).collect::<Vec<_>>().join(","))
            .unwrap();
        for element in html.select(&selector) {
            let mut reader = self.agent.get(element.value().attr("href").unwrap()).call()?.into_reader();
            io::copy(&mut reader,
                &mut File::create(path.as_ref().join(format!("{}.jpg", element.value().id().unwrap())))?)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "aaData")]
    aa_data: Vec<Vec<String>>,
}

impl MetadataFetcher for MetalArchives {
    fn download_artist_logo_and_photo(&self, artist: &str, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let mut uri = self.base_uri.join("/search/ajax-band-search")?;
        uri.query_pairs_mut().append_pair("field", "name").append_pair("query", artist);
        let search_response = self.get_search_response(&uri)?;
        if let [band_uri_fragment, ..] = &search_response[..] {
            self.download(band_uri_fragment, vec!["logo", "photo"], path)
        } else {
            Err(anyhow!("band uri fragment not found [{:?}]", search_response))
        }
    }
    fn download_cover(&self, artist: &str, album: &str, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let mut uri = self.base_uri.join("/search/ajax-advanced/searching/albums")?;
        uri.query_pairs_mut().append_pair("bandName", artist).append_pair("releaseTitle", album);
        let search_response = self.get_search_response(&uri)?;
        if let [_, album_uri_fragment, ..] = &search_response[..] {
            self.download(album_uri_fragment, vec!["cover"], path)
        } else {
            Err(anyhow!("album uri fragment not found [{:?}]", search_response))
        }
    }
}

#[test]
fn main() {
    let archives = MetalArchives::new();
    if let Err(error) = archives.download_artist_logo_and_photo("Haken", "/home/dusk/Desktop/") {
        println!("error {}", error);
    }
    if let Err(error) = archives.download_cover("Haken", "affinity", "/home/dusk/Desktop/") {
        println!("error {}", error);
    }
}
