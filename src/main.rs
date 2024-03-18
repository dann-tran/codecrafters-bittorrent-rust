use anyhow::Context;
use clap::{Parser, Subcommand};

use bittorrent_starter_rust::{
    decode::decode_bencoded_value,
    torrent::Torrent,
    tracker::{TrackerRequest, TrackerResponse},
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Decode { value: String },
    Info { filepath: PathBuf },
    Peers { filepath: PathBuf },
}

fn urlencode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(3 * bytes.len());
    for &byte in bytes {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Decode { value } => {
            let decoded_value = decode_bencoded_value(&value);
            println!("{}", decoded_value.to_string());
        }
        Command::Info { filepath } => {
            let content = std::fs::read(filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)?;
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);

            let info_hash = torrent.info_hash();
            println!("Info Hash: {}", hex::encode(&info_hash));

            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            torrent.info.pieces.chunks_exact(20).for_each(|chunk| {
                println!("{}", hex::encode(chunk));
            })
        }
        Command::Peers { filepath } => {
            let content = std::fs::read(filepath)?;
            let torrent =
                serde_bencode::from_bytes::<Torrent>(&content).expect("Torrent is serializable");

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
            let tracker_res = serde_bencode::from_bytes::<TrackerResponse>(&res)
                .context("Parse TrackerResponse")?;

            tracker_res.peers.chunks_exact(6).for_each(|chunk| {
                println!(
                    "{}.{}.{}.{}:{}",
                    chunk[0],
                    chunk[1],
                    chunk[2],
                    chunk[3],
                    ((chunk[4] as u16) << 8 | chunk[5] as u16)
                )
            })
        }
    }
    Ok(())
}
