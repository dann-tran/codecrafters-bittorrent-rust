use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::{self, Map};
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
}

fn _decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match encoded_value.chars().next() {
        Some('i') => {
            // Example: "i52e" -> 52
            if let Some((n, remainder)) =
                (&encoded_value[1..])
                    .split_once('e')
                    .and_then(|(digits, remainder)| {
                        // ok() converts Result<T, E> to Option<T>
                        // ? causes the function the return None if None, else unwraps the value
                        let n = digits.parse::<i64>().ok()?;
                        Some((n, remainder))
                    })
            {
                return (n.into(), remainder);
            }
        }
        Some(c) if c.is_ascii_digit() => {
            // Example: "5:hello" -> "hello"
            if let Some((string, remainder)) =
                encoded_value
                    .split_once(':')
                    .and_then(|(digits, remainder)| {
                        let n = digits.parse::<usize>().ok()?;
                        Some((remainder[..n].to_string(), &remainder[n..]))
                    })
            {
                return (string.into(), remainder);
            }
        }
        Some('l') => {
            // Example: "l5:helloi52ee" -> ["hello", 52]
            let mut values = Vec::new();
            let mut remainder = &encoded_value[1..];
            while !remainder.is_empty() && !remainder.starts_with('e') {
                let (val, _remainder) = _decode_bencoded_value(remainder);
                values.push(val);
                remainder = _remainder;
            }
            return (values.into(), &remainder[1..]);
        }
        Some('d') => {
            // Example: "d3:foo3:bar5:helloi52ee" -> {"hello": 52, "foo":"bar"}
            let mut dict = Map::new();
            let mut remainder = &encoded_value[1..];
            while !remainder.is_empty() && !remainder.starts_with('e') {
                let (key, _remainder) = _decode_bencoded_value(remainder);
                remainder = _remainder;
                let (val, _remainder) = _decode_bencoded_value(remainder);
                remainder = _remainder;
                let key = match key {
                    serde_json::Value::String(k) => k,
                    k => panic!("Key must be a string, not {k:?}"),
                };
                dict.insert(key, val);
            }
            return (dict.into(), &remainder[1..]);
        }
        _ => {}
    }
    panic!("Unhandled encoded value")
}

fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    let (val, remainder) = _decode_bencoded_value(encoded_value);
    if !remainder.is_empty() {
        eprintln!("Extra remainder: {remainder}");
        panic!("Invalid encoded value: {encoded_value}")
    }
    return val;
}

#[derive(Debug, Deserialize, Serialize)]
struct Info {
    length: usize,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: usize,
    pieces: ByteBuf,
}

#[derive(Debug, Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
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
                let chunk = chunk.iter().map(|b| format!("{:x}", b)).collect::<String>();
                println!("{}", chunk);
            })
        }
    }
    Ok(())
}
