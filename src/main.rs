use serde_json::{self, Map};
use sha1::{Digest, Sha1};
use std::env;

// Available if you need it!
// use serde_bencode;
const DECODE_COMMAND: &str = "decode";
// const INFO_COMMAND: &str = "info";

fn _decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    eprintln!("Decoding {encoded_value}");
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

fn read_metainfo(filepath: &str) -> Map<String, serde_json::Value> {
    let content = std::fs::read(filepath).expect("Invalid file");
    let mut chars = content.iter();
    match chars.next() {
        Some(b'd') => _decode_bencoded_dictionary(&mut chars),
        Some(_) | None => {
            panic!("Invalid metainfo")
        }
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    match command.as_str() {
        DECODE_COMMAND => {
            let encoded_value = &args[2];
            let decoded_value = decode_bencoded_value(encoded_value);
            println!("{}", decoded_value.to_string());
        }
        INFO_COMMAND => {
            let filepath = &args[2];
            let dict = read_metainfo(filepath);
            println!(
                "Tracker URL: {}",
                dict.get("announce")
                    .expect("Missing `announce` key")
                    .as_str()
                    .unwrap()
            );
            let info_dict = dict
                .get("info")
                .expect("Missing `info` key")
                .as_object()
                .unwrap();
            println!(
                "Length: {}",
                info_dict.get("length").expect("Missing `length` key")
            );

            let mut hasher = Sha1::new();
            let encoded_info = serde_bencode::to_bytes(info_dict).unwrap();
            hasher.update(encoded_info);
            let result = hasher.finalize();
            println!("Info Hash: {:x}", result)
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
        _ => {}
    }
}
