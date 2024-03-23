use anyhow::Context;
use clap::{arg, Parser, Subcommand};

use bittorrent_starter_rust::{
    decode::decode_bencoded_value, download::download_piece, handshake::perform_handshake,
    message::MessageFramer, torrent::Torrent, tracker::request_tracker,
};
use std::path::PathBuf;
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum Command {
    Decode {
        value: String,
    },
    Info {
        filepath: PathBuf,
    },
    Peers {
        filepath: PathBuf,
    },
    Handshake {
        filepath: PathBuf,
        peer_addr: String,
    },
    DownloadPiece {
        #[arg(short)]
        outpath: PathBuf,
        filepath: PathBuf,
        piece_index: usize,
    },
    Download {
        #[arg(short)]
        outpath: PathBuf,
        filepath: PathBuf,
    },
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
            torrent.info.piece_hashes().iter().for_each(|chunk| {
                println!("{}", hex::encode(chunk));
            })
        }
        Command::Peers { filepath } => {
            let content = std::fs::read(filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)
                .context("Deserialize torrent file")?;

            let tracker_res = request_tracker(&torrent).await?;
            tracker_res.get_peers().iter().for_each(|ip_addr| {
                println!("{}", ip_addr);
            })
        }
        Command::Handshake {
            filepath,
            peer_addr,
        } => {
            let content = std::fs::read(filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)
                .context("Deserialize torrent file")?;

            let mut tcp_stream = TcpStream::connect(&peer_addr).await?;
            let peer_msg = perform_handshake(&torrent, &mut tcp_stream).await?;
            println!("Peer ID: {}", hex::encode(&peer_msg.peer_id));
        }
        Command::DownloadPiece {
            outpath,
            filepath,
            piece_index,
        } => {
            let content = std::fs::read(&filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)
                .context("Deserialize torrent file")?;

            let tracker_res = request_tracker(&torrent).await?;
            let peers = tracker_res.get_peers();
            let peer_addr = &peers.iter().nth(0).context("Get peer addr")?;
            let mut tcp_stream = TcpStream::connect(&peer_addr).await?;

            perform_handshake(&torrent, &mut tcp_stream).await?;
            let mut framed = tokio_util::codec::Framed::new(&mut tcp_stream, MessageFramer);
            let piece_bytes = download_piece(&torrent, &mut framed, piece_index).await?;

            tokio::fs::write(&outpath, &piece_bytes)
                .await
                .context("write out downloaded piece")?;
            println!("Piece {piece_index} downloaded to {}.", outpath.display());
        }
        Command::Download { outpath, filepath } => {
            let content = std::fs::read(&filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)
                .context("Deserialize torrent file")?;

            let tracker_res = request_tracker(&torrent).await?;
            let peers = tracker_res.get_peers();
            let peer_addr = &peers.iter().nth(0).context("Get peer addr")?;
            let mut tcp_stream = TcpStream::connect(&peer_addr).await?;

            let mut framed = tokio_util::codec::Framed::new(&mut tcp_stream, MessageFramer);
            let mut file_bytes = Vec::with_capacity(torrent.info.piece_length);
            for piece_index in 0..torrent.info.length.div_ceil(torrent.info.piece_length) {
                eprintln!("Downloading piece {piece_index}");
                let piece_bytes = download_piece(&torrent, &mut framed, piece_index).await?;
                file_bytes.extend(piece_bytes);
            }

            tokio::fs::write(&outpath, &file_bytes)
                .await
                .context("write out downloaded file")?;
            println!(
                "Downloaded {} to {}.",
                filepath.display(),
                outpath.display()
            );
        }
    }
    Ok(())
}
