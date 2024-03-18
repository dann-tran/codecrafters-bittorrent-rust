use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

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

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
        let bencoded_info = serde_bencode::to_bytes(&self.info).expect("info is serializable");
        let mut hasher = Sha1::new();
        hasher.update(&bencoded_info);
        hasher.finalize().into()
    }
}
