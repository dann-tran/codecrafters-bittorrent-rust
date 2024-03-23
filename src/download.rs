use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::message::{
    Message, MessageFramer, MessageTag, PieceMessagePayload, RequestMessagePayload,
};
use crate::torrent::Torrent;
use crate::utils::compute_hash;

pub async fn download_piece(
    torrent: &Torrent,
    framed: &mut Framed<&mut TcpStream, MessageFramer>,
    piece_index: usize,
) -> anyhow::Result<Vec<u8>> {
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
            RequestMessagePayload::new(piece_index as u32, offset as u32, block_size as u32);
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

        let piece = PieceMessagePayload::ref_from_bytes(&piece.payload[..])
            .expect("always get all Piece response fields from peer");
        assert_eq!(piece.index() as usize, piece_index);
        assert_eq!(piece.begin() as usize, offset);
        assert_eq!(piece.block().len(), block_size);
        piece_bytes.extend(piece.block());
    }

    assert_eq!(piece_bytes.len(), piece_size);

    let hash = compute_hash(&piece_bytes);
    let piece_hashes = torrent.info.piece_hashes();
    let piece_hash = piece_hashes
        .iter()
        .nth(piece_index)
        .context("Piece index is valid")?;

    assert_eq!(&hash, piece_hash);

    Ok(piece_bytes)
}
