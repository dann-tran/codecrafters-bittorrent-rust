use anyhow::Context;
use clap::{arg, Parser, Subcommand};

use bittorrent_starter_rust::{
    decode::decode_bencoded_value, handshake::perform_handshake, torrent::Torrent,
    tracker::request_tracker,
};
use std::path::PathBuf;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
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
    // DownloadPiece {
    //     #[arg(short)]
    //     outpath: PathBuf,
    //     filepath: PathBuf,
    //     piece_index: usize,
    // },
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
        } // Command::DownloadPiece {
          //     outpath,
          //     filepath,
          //     piece_index,
          // } => {
          //     let content = std::fs::read(filepath)?;
          //     let torrent = serde_bencode::from_bytes::<Torrent>(&content)
          //         .context("Deserialize torrent file")?;

          //     let tracker_res = request_tracker(&torrent).await?;
          //     let peers = tracker_res.get_peers();
          //     let peer_addr = &peers.iter().nth(0).context("Get peer addr")?;

          //     let mut tcp_stream = TcpStream::connect(&peer_addr).await?;
          //     let peer_msg = perform_handshake(&torrent, &mut tcp_stream).await?;

          //     // receive bitfield
          //     let msg = read_peer_message(&mut tcp_stream).await?;
          //     assert_eq!(msg.id, b'5');

          //     // send interested
          //     let mut msg: [u8; 5] = [0; 5];
          //     msg[4] = 2;
          //     tcp_stream.write_all(&msg).await?;

          //     tcp_stream.read_exact(&mut length).await?;
          //     let msg_length = compute_4byte_int(&length);
          //     let payload_length = msg_length - 5;

          //     let mut payload = vec![0; payload_length];
          //     tcp_stream.read_exact(&mut payload).await?;

          //     // recieve unchoke
          //     let id = tcp_stream.read_u8().await?;
          //     assert_eq!(id, b'1');

          //     const BLOCK_SIZE: usize = 2 << 14;
          //     const REQUEST_MSG_LENGTH: usize = 4 + 1 + 4 + 4 + 4;
          //     let piece: Vec<&[u8; BLOCK_SIZE]> = Vec::new();

          //     for offset in (0..torrent.info.piece_length).step_by(BLOCK_SIZE) {
          //         // send rqeuest
          //         let mut msg: [u8; REQUEST_MSG_LENGTH] = [0; REQUEST_MSG_LENGTH];
          //         msg[..4].copy_from_slice(&REQUEST_MSG_LENGTH.to_le_bytes()); // message length
          //         msg[4] = 6; // message id
          //         msg[5..9].copy_from_slice(&piece_index.to_le_bytes()); // piece index
          //         msg[9..13].copy_from_slice(&offset.to_le_bytes()); // byte offset
          //         let block_length =
          //             std::cmp::min(offset + BLOCK_SIZE, torrent.info.piece_length) - offset;
          //         msg[13..].copy_from_slice(&block_length.to_le_bytes()); // block length

          //         tcp_stream.write_all(&msg).await?;

          //         // receive piece

          //         let id = tcp_stream.read_u8().await?;
          //         assert_eq!(id, b'7');
          //         let mut block: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
          //         tcp_stream.read_exact(&block).await?;
          //     }
          // }
    }
    Ok(())
}
