use clap::{Parser, Subcommand};
use reqwest::blocking::Client;

use bittorrent_starter_rust::{decode::decode_bencoded_value, torrent::Torrent};
use sha1::{Digest, Sha1};
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

fn main() -> anyhow::Result<()> {
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

            let bencoded_info = serde_bencode::to_bytes(&torrent.info)?;
            let mut hasher = Sha1::new();
            hasher.update(bencoded_info);
            println!("Info Hash: {:x}", hasher.finalize());

            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            torrent.info.pieces.chunks_exact(20).for_each(|chunk| {
                println!("{}", hex::encode(chunk));
            })
        }
        Command::Peers { filepath } => {
            let content = std::fs::read(filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)?;
            let client = Client::new();

            let bencoded_info = serde_bencode::to_bytes(&torrent.info)?;
            let mut hasher = Sha1::new();
            hasher.update(bencoded_info);
            let info_hash = hasher.finalize();
        }
    }
    Ok(())
}
