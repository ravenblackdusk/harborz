use std::collections::HashMap;
use std::sync::mpsc::Sender;
use anyhow::anyhow;
use bytes::Bytes;
use futures::{FutureExt, TryFutureExt};
use futures::future::BoxFuture;
use libproxy::ProxyFactory;
use scraper::{ElementRef, Html, Selector};
use serde_derive::Deserialize;
use url::Url;
use metadata_fetch::{AlbumSearch, ArtistSearch, DownloadAlbumEvent, DownloadArtistEvent, MetadataFetcher};
use metadata_fetch::DownloadAlbumEvent::*;
use metadata_fetch::DownloadArtistEvent::*;
use reqwest::{Client, Proxy, Response};
use scraper::node::Element;
use async_std::task;

pub struct MetalArchives {
    base_uri: Url,
    client: Client,
}

impl MetalArchives {
    pub fn new() -> Self {
        const BASE_URI: &'static str = "https://www.metal-archives.com";
        Self {
            base_uri: Url::parse(BASE_URI).unwrap(),
            client: if let [proxy, ..] = &ProxyFactory::new().unwrap().get_proxies(BASE_URI).unwrap()[..] {
                Client::builder().proxy(Proxy::all(proxy).unwrap()).build().unwrap()
            } else {
                Client::new()
            },
        }
    }
    async fn get_search_response(&self, uri: Url) -> anyhow::Result<Vec<Vec<String>>> {
        Ok(self.client.get(uri.as_str()).send().await?.json::<SearchResponse>().await?.aa_data.into_iter().take(4)
            .collect::<Vec<_>>())
    }
    fn text(element_ref: ElementRef) -> String {
        element_ref.first_child().unwrap().value().as_text().unwrap().to_string()
    }
    fn a() -> Selector {
        Selector::parse("a").unwrap()
    }
    async fn download<'a>(&'a self, a: Element, ids: Vec<&'a str>)
        -> anyhow::Result<Vec<Option<BoxFuture<anyhow::Result<Bytes>>>>> {
        let uri = a.attr("href").unwrap();
        let response = async { self.client.get(uri).send().await?.text().await }.await
            .map_err(|it| { anyhow!("error [{it}]") })?;
        let html = Html::parse_document(&response);
        let selector = Selector::parse(&ids.iter().map(|id| { format!("#{id}") }).collect::<Vec<_>>().join(","))
            .map_err(|it| { anyhow!("error [{it}]") })?;
        let mut id_to_read = html.select(&selector).into_iter().map(|element| {
            (element.value().id().unwrap(), self.client.get(element.value().attr("href").unwrap()).send()
                .and_then(Response::bytes).map_err(|error| { anyhow!("error {error}") }).boxed())
        }).collect::<HashMap<_, _>>();
        Ok(ids.into_iter().map(|id| { id_to_read.remove(id) }).collect::<Vec<_>>())
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "aaData")]
    aa_data: Vec<Vec<String>>,
}

impl MetadataFetcher for MetalArchives {
    fn download_artist_logo_and_photo(&'static self, artist: &str, sender: Sender<DownloadArtistEvent>) {
        let mut uri = self.base_uri.join("/search/ajax-band-search").unwrap();
        uri.query_pairs_mut().append_pair("field", "name").append_pair("query", artist);
        task::spawn(self.get_search_response(uri).map(move |search_response| {
            sender.send(DownloadArtistEvent::SearchResult(search_response.map(|search_response| {
                search_response.into_iter().enumerate().map({
                    let sender = sender.clone();
                    move |(i, mut it)| {
                        if let [_, _, _] = it[..] {
                            let location = it.pop().unwrap();
                            let genre = it.pop().unwrap();
                            let band_fragment = Html::parse_fragment(&it.pop().unwrap());
                            let a = band_fragment.select(&Self::a()).next().unwrap();
                            let name = Self::text(a);
                            task::spawn_local(self.download(a.value().to_owned(), vec!["logo", "photo"]).map_ok({
                                let sender = sender.clone();
                                move |mut images| {
                                    if let Some(photo) = images.pop().unwrap() {
                                        task::spawn(photo.map({
                                            let sender = sender.clone();
                                            move |photo| { sender.send(Photo(i, photo)).unwrap(); }
                                        }));
                                    }
                                    if let Some(logo) = images.pop().unwrap() {
                                        task::spawn(logo.map(move |logo| { sender.send(Logo(i, logo)).unwrap(); }));
                                    }
                                }
                            }));
                            Ok(ArtistSearch {
                                name,
                                genre,
                                location,
                            })
                        } else {
                            Err(anyhow!("unexpected band search response [{it:?}]"))
                        }
                    }
                }).collect::<Vec<_>>()
            }))).unwrap();
        }));
    }
    fn download_cover(&'static self, artist: &str, album: &str, sender: Sender<DownloadAlbumEvent>) {
        let mut uri = self.base_uri.join("/search/ajax-advanced/searching/albums").unwrap();
        uri.query_pairs_mut().append_pair("bandName", artist).append_pair("releaseTitle", album);
        task::spawn(self.get_search_response(uri).map(move |search_response| {
            sender.send(DownloadAlbumEvent::SearchResult(search_response.map(|search_response| {
                search_response.into_iter().enumerate().map({
                    let sender = sender.clone();
                    move |(i, mut it)| {
                        if let [_, _, _] = it[..] {
                            let album_type = it.pop().unwrap();
                            let album_fragment = Html::parse_fragment(&it.pop().unwrap());
                            let artist_fragment = Html::parse_fragment(&it.pop().unwrap());
                            let a = album_fragment.select(&Self::a()).next().unwrap();
                            let album = Self::text(a);
                            task::spawn_local(self.download(a.value().to_owned(), vec!["cover"]).map_ok({
                                let sender = sender.clone();
                                move |mut cover| {
                                    if let Some(cover) = cover.pop().unwrap() {
                                        task::spawn(cover.map(move |cover| { sender.send(Cover(i, cover)).unwrap(); }));
                                    }
                                }
                            }));
                            Ok(AlbumSearch {
                                artist: Self::text(artist_fragment.select(&Self::a()).next().unwrap()),
                                album,
                                album_type,
                            })
                        } else {
                            Err(anyhow!("unexpected album search response [{it:?}]"))
                        }
                    }
                }).collect::<Vec<_>>()
            }))).unwrap();
        }));
    }
}
