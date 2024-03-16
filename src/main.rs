use serde_json::{self, Map};
use std::{env, slice::Iter};

// Available if you need it!
// use serde_bencode
const DECODE_COMMAND: &str = "decode";
const INFO_COMMAND: &str = "info";

fn _decode_bencoded_integer(chars: &mut Iter<u8>) -> i64 {
    // Example: "(i)52e" -> 52
    let mut number_chars: Vec<u8> = Vec::new();
    loop {
        match chars.next() {
            Some(b'e') => {
                break;
            }
            Some(&c) => {
                number_chars.push(c);
            }
            None => {
                panic!("Invalid bencoded integer")
            }
        }
    }
    String::from_utf8(number_chars.into_iter().collect::<Vec<u8>>())
        .unwrap()
        .parse::<i64>()
        .expect("Invalid integer")
}

fn _decode_bencoded_string(c: u8, chars: &mut Iter<u8>) -> String {
    // Example: "5:hello" -> "hello"
    let mut number_chars: Vec<u8> = vec![c];
    loop {
        match chars.next() {
            Some(b':') => {
                break;
            }
            Some(&c) if c.is_ascii_digit() => {
                number_chars.push(c);
            }
            Some(_) | None => {
                panic!("Invalid bencoded string")
            }
        }
    }
    let number_str = String::from_utf8(number_chars.into_iter().collect::<Vec<u8>>()).unwrap();
    let number = number_str
        .parse::<usize>()
        .expect(format!("Invalid number: {}\n", number_str).as_str());
    String::from_utf8_lossy(&chars.take(number).cloned().collect::<Vec<u8>>()).into_owned()
}

fn _decode_bencoded_list(chars: &mut Iter<u8>) -> Vec<serde_json::Value> {
    // Example: "(l)5:helloi52ee" -> ["hello", 52]
    let mut vec: Vec<serde_json::Value> = Vec::new();
    loop {
        match chars.next() {
            Some(b'e') => {
                break;
            }
            Some(&_c) => {
                let val = _decode_bencoded_value(_c, chars);
                vec.push(val);
            }
            None => {
                panic!("Invalid bencoded list")
            }
        }
    }
    vec
}

fn _decode_bencoded_dictionary(chars: &mut Iter<u8>) -> Map<String, serde_json::Value> {
    // Example: "(d)3:foo3:bar5:helloi52ee" -> {"hello": 52, "foo":"bar"}
    let mut dict: Map<String, serde_json::Value> = Map::new();
    loop {
        match chars.next() {
            Some(b'e') => {
                break;
            }
            Some(&c) => {
                let key = _decode_bencoded_string(c, chars);
                let val = _decode_bencoded_value(*chars.next().unwrap(), chars);
                dict.insert(key, val);
            }
            None => {
                panic!("Invalid dict")
            }
        }
    }
    dict
}

fn _decode_bencoded_value(c: u8, chars: &mut Iter<u8>) -> serde_json::Value {
    match c {
        b'i' => serde_json::Value::Number(_decode_bencoded_integer(chars).into()),
        _c if _c.is_ascii_digit() => serde_json::Value::String(_decode_bencoded_string(c, chars)),
        b'l' => serde_json::Value::Array(_decode_bencoded_list(chars)),
        b'd' => serde_json::Value::Object(_decode_bencoded_dictionary(chars)),
        _ => {
            panic!("Unhandled encoded value")
        }
    }
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    let mut chars = encoded_value.as_bytes().iter();
    match chars.next() {
        Some(&c) => _decode_bencoded_value(c, &mut chars),
        None => {
            panic!("Unhandled encoded value {}", encoded_value)
        }
    }
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
            println!(
                "Length: {}",
                dict.get("info")
                    .expect("Missing `info` key")
                    .as_object()
                    .unwrap()
                    .get("length")
                    .expect("Missing `length` key")
            );
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
