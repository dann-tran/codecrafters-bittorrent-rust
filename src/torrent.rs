use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub length: usize,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    pub pieces: ByteBuf,
}

#[derive(Debug, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}
