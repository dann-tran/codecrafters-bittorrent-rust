use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use crate::torrent::Torrent;

#[derive(Debug, Serialize)]
pub struct TrackerRequest {
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub compact: u8,
}

#[derive(Debug, Deserialize)]
pub struct TrackerResponse {
    pub interval: i64,
    pub peers: ByteBuf,
}

impl TrackerResponse {
    pub fn get_peers(&self) -> Vec<String> {
        self.peers
            .chunks_exact(6)
            .map(|chunk| {
                format!(
                    "{}.{}.{}.{}:{}",
                    chunk[0],
                    chunk[1],
                    chunk[2],
                    chunk[3],
                    ((chunk[4] as u16) << 8 | chunk[5] as u16)
                )
            })
            .collect()
    }
}

fn urlencode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(3 * bytes.len());
    for &byte in bytes {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

pub async fn request_tracker(torrent: &Torrent) -> anyhow::Result<TrackerResponse> {
    let tracker_req = TrackerRequest {
        peer_id: String::from("00112233445566778899"),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: torrent.info.length,
        compact: 1,
    };
    let url_params =
        serde_urlencoded::to_string(&tracker_req).context("URL-encode TrackRequest")?;
    let url = format!(
        "{}?info_hash={}&{}",
        &torrent.announce,
        &urlencode(&torrent.info_hash()),
        &url_params
    );

    let res = reqwest::get(&url).await?.bytes().await?;
    serde_bencode::from_bytes::<TrackerResponse>(&res).context("Parse TrackerResponse")
}
