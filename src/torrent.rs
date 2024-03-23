use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use crate::utils::compute_hash;

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub length: usize, // size of torrent file in bytes
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: usize, // number of bytes in each piece
    pub pieces: ByteBuf, // concatenated SHA-1 hashes of each piece
}

impl Info {
    pub fn piece_hashes(&self) -> Vec<[u8; 20]> {
        self.pieces
            .chunks_exact(20)
            .map(|chunk| chunk.try_into().expect("Chunk to be of size 20"))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
        let bencoded_info = serde_bencode::to_bytes(&self.info).expect("info is serializable");
        compute_hash(&bencoded_info)
    }
}
