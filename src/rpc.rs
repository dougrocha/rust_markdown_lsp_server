use std::io::{BufRead, Write};

use log::debug;

use crate::message::Message;

pub fn encode_message<T: serde::Serialize>(msg: T) -> Result<String, serde_json::Error> {
    let result_str = serde_json::to_string(&msg)?;

    let output = format!("Content-Length: {}\r\n\r\n{}", result_str.len(), result_str);

    Ok(output)
}

pub fn write_msg(out: &mut dyn Write, msg: &str) -> std::io::Result<()> {
    debug!("Sending: {:?}", msg);

    write!(out, "{}", msg.len())?;
    out.write_all(msg.as_bytes())?;
    out.flush()?;

    Ok(())
}

pub fn read_message<R: BufRead>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut buf = String::new();

    loop {
        let mut line = String::new();

        if reader.read_line(&mut line)? == 0 {
            if buf.is_empty() {
                return Ok(None);
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "EOF before end of headers",
                ));
            }
        }

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
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Content-Length header not found",
            )
        })?;

    let mut buf = buf.into_bytes();
    buf.resize(content_length, 0);
    reader.read_exact(&mut buf)?;

    let buf = String::from_utf8(buf).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Content-Length header not found",
        )
    })?;

    Ok(Some(buf))
}

pub fn handle_message<R: BufRead>(reader: &mut R) -> Result<Message, String> {
    let content = read_message(reader)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No message received".to_string())?;

    let message: Message = match serde_json::from_str(&content) {
        Ok(msg) => msg,
        Err(e) => {
            return Err(format!("Failed to parse message: {:?}\n{:?}", e, content));
        }
    };

    Ok(message)
}
