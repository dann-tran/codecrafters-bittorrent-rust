use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    let mut chars = encoded_value.chars();
    match chars.next() {
        Some('i') => {
            // Example: "i52e" -> 52
            assert_eq!(chars.last(), Some('e'), "Invalid bencoded integer");
            let number_string = &encoded_value[1..encoded_value.len() - 1];
            let number: isize = number_string.parse().expect("Invalid number");
            serde_json::Value::Number(number.into())
        }
        Some(c) if c.is_digit(10) => {
            // Example: "5:hello" -> "hello"
            let colon_index = encoded_value.find(':').expect("Missing colon");
            let number_string = &encoded_value[..colon_index];
            let number = number_string.parse::<i64>().expect("Invalid number");
            let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
            serde_json::Value::String(string.to_string())
        }
        Some(_) | None => {
            panic!("Unhandled encoded value: {}", encoded_value)
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
