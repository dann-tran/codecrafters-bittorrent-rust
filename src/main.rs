use serde_json;
use std::{env, str::Chars};

// Available if you need it!
// use serde_bencode

fn _decode_bencoded_value(c: char, chars: &mut Chars<'_>) -> serde_json::Value {
    match c {
        'i' => {
            // Example: "i52e" -> 52
            let mut number_chars: Vec<char> = Vec::new();
            loop {
                match chars.next() {
                    Some('e') => {
                        break;
                    }
                    Some(c) if c.is_ascii_digit() => {
                        number_chars.push(c);
                    }
                    Some(_) | None => {
                        panic!("Invalid bencoded integer")
                    }
                }
            }
            let number = number_chars
                .into_iter()
                .collect::<String>()
                .parse::<i64>()
                .expect("Invalid number");
            serde_json::Value::Number(number.into())
        }
        _c if _c.is_digit(10) => {
            // Example: "5:hello" -> "hello"
            let mut number_chars: Vec<char> = vec![c];
            loop {
                match chars.next() {
                    Some(':') => {
                        break;
                    }
                    Some(c) if c.is_ascii_digit() => {
                        number_chars.push(c);
                    }
                    Some(_) | None => {
                        panic!("Invalid bencoded string")
                    }
                }
            }
            let number = number_chars
                .into_iter()
                .collect::<String>()
                .parse::<usize>()
                .expect("Invalid number");
            let string = chars.take(number).collect::<String>();
            serde_json::Value::String(string)
        }
        'l' => {
            // Example: "l5:helloi52ee" -> ["hello", 52]
            let mut vec: Vec<serde_json::Value> = Vec::new();
            loop {
                match chars.next() {
                    Some('e') => {
                        break;
                    }
                    Some(_c) => {
                        let val = _decode_bencoded_value(_c, chars);
                        vec.push(val);
                    }
                    None => {
                        panic!("Invalid bencoded list")
                    }
                }
            }
            serde_json::Value::Array(vec)
        }
        _ => {
            panic!("Unhandled encoded value")
        }
    }
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    let mut chars = encoded_value.chars();
    match chars.next() {
        Some(c) => _decode_bencoded_value(c, &mut chars),
        None => {
            panic!("Unhandled encoded value {}", encoded_value)
        }
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
