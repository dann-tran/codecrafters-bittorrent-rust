use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::torrent::Torrent;

#[derive(Debug)]
#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Self {
        Self {
            length: 19,
            protocol: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash: *info_hash,
            peer_id: *peer_id,
        }
    }
}

pub async fn perform_handshake(
    torrent: &Torrent,
    tcp_stream: &mut TcpStream,
) -> anyhow::Result<Handshake> {
    let info_hash = torrent.info_hash();
    let mut handshake = Handshake::new(&info_hash, &b"00112233445566778899");

    let bytes = &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];
    let bytes: &mut [u8; std::mem::size_of::<Handshake>()] = unsafe { &mut *bytes };

    tcp_stream
        .write_all(bytes)
        .await
        .context("Write handshake")?;
    tcp_stream
        .read_exact(bytes)
        .await
        .context("Read handshake")?;

    Ok(handshake)
}
