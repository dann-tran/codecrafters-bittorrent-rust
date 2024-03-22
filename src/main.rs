use anyhow::Context;
use clap::{arg, Parser, Subcommand};

use bittorrent_starter_rust::{
    decode::decode_bencoded_value,
    handshake::perform_handshake,
    message::{Message, MessageFramer, MessageTag, Piece, RequestPayload},
    torrent::Torrent,
    tracker::request_tracker,
};
use futures_util::{SinkExt, StreamExt};
use sha1::{Digest, Sha1};
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
            let content = std::fs::read(filepath)?;
            let torrent = serde_bencode::from_bytes::<Torrent>(&content)
                .context("Deserialize torrent file")?;

            let tracker_res = request_tracker(&torrent).await?;
            let peers = tracker_res.get_peers();
            let peer_addr = &peers.iter().nth(0).context("Get peer addr")?;
            let mut tcp_stream = TcpStream::connect(&peer_addr).await?;
            perform_handshake(&torrent, &mut tcp_stream).await?;

            let mut framed = tokio_util::codec::Framed::new(tcp_stream, MessageFramer);
            let bitfield = framed
                .next()
                .await
                .expect("peer always sends a bitfields")
                .context("peer message was invalid")?;
            assert_eq!(bitfield.tag, MessageTag::Bitfield);
            // NOTE: we assume that the bitfield covers all pieces

            framed
                .send(Message {
                    tag: MessageTag::Interested,
                    payload: Vec::new(),
                })
                .await
                .context("send interested message")?;

            let unchoke = framed
                .next()
                .await
                .expect("peer always sends an unchoke")
                .context("peer message was invalid")?;
            assert_eq!(unchoke.tag, MessageTag::Unchoke);
            assert!(unchoke.payload.is_empty());

            let piece_size = if piece_index == torrent.info.pieces.chunks_exact(20).len() - 1 {
                let size = torrent.info.length % torrent.info.piece_length;
                if size == 0 {
                    torrent.info.piece_length
                } else {
                    size
                }
            } else {
                torrent.info.piece_length
            };
            let mut piece_bytes: Vec<u8> = Vec::with_capacity(piece_size);
            for (block_index, offset) in (0..piece_size).step_by(1 << 14).enumerate() {
                let block_size = std::cmp::min(&piece_size - offset, 1 << 14);
                let mut req =
                    RequestPayload::new(piece_index as u32, offset as u32, block_size as u32);
                let request_bytes = Vec::from(req.as_bytes_mut());
                framed
                    .send(Message {
                        tag: MessageTag::Request,
                        payload: request_bytes,
                    })
                    .await
                    .with_context(|| format!("send request for block {block_index}"))?;

                let piece = framed
                    .next()
                    .await
                    .expect("peer always sends a piece")
                    .context("peer message was invalid")?;
                assert_eq!(piece.tag, MessageTag::Piece);
                assert!(!piece.payload.is_empty());

                let piece = Piece::ref_from_bytes(&piece.payload[..])
                    .expect("always get all Piece response fields from peer");
                assert_eq!(piece.index() as usize, piece_index);
                assert_eq!(piece.begin() as usize, offset);
                assert_eq!(piece.block().len(), block_size);
                piece_bytes.extend(piece.block());
            }

            assert_eq!(piece_bytes.len(), piece_size);

            let mut hasher = Sha1::new();
            hasher.update(&piece_bytes);
            let hash: [u8; 20] = hasher
                .finalize()
                .try_into()
                .expect("GenericArray<_, 20> == [_; 20]");

            let piece_hashes = torrent.info.piece_hashes();
            let piece_hash = piece_hashes
                .iter()
                .nth(piece_index)
                .context("Piece index is valid")?;

            assert_eq!(&hash, piece_hash);

            tokio::fs::write(&outpath, piece_bytes)
                .await
                .context("write out downloaded piece")?;
            println!("Piece {piece_index} downloaded to {}.", outpath.display());
        }
    }
    Ok(())
}
