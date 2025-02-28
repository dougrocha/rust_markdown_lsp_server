use std::io::BufRead;

use serde_json::Value;

use crate::message::Message;

pub fn encode_message<T: serde::Serialize>(msg: T) -> String {
    let result_str = serde_json::to_string(&msg).unwrap();

    let output = format!("Content-Length: {}\r\n\r\n{}", result_str.len(), result_str);

    output
}

pub fn handle_message<R: BufRead>(reader: &mut R) -> Message {
    let mut buf = String::new();

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        buf.push_str(&line);
        if line == "\r\n" {
            break;
        }
    }

    let content_length = buf
        .lines()
        .find(|line| line.starts_with("Content-Length: "))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|len| len.trim().parse::<usize>().ok())
        .unwrap();

    let mut content = vec![0; content_length];
    reader.read_exact(&mut content).unwrap();

    let message: Value = serde_json::from_slice(&content).unwrap();

    // notification will not have id
    if message.get("id").is_some() {
        Message::Request(serde_json::from_value(message).unwrap())
    } else {
        Message::Notification(serde_json::from_value(message).unwrap())
    }
}
