use std::io::{BufRead, Write};

use miette::{miette, Context, IntoDiagnostic, Result};

use crate::message::Message;

pub fn encode_message<T: serde::Serialize>(msg: T) -> Result<String> {
    let result_str = serde_json::to_string(&msg)
        .into_diagnostic()
        .map_err(|e| miette!("Failed to serialize message to JSON: {}", e))?;

    let output = format!("Content-Length: {}\r\n\r\n{}", result_str.len(), result_str);

    Ok(output)
}

pub fn write_msg(out: &mut dyn Write, msg: &str) -> Result<()> {
    out.write_all(msg.as_bytes())
        .into_diagnostic()
        .wrap_err("Failed to write message to output stream")?;
    out.flush()
        .into_diagnostic()
        .wrap_err("Failed to flush output stream")?;

    Ok(())
}

pub fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<String>> {
    let mut buf = String::new();

    loop {
        let mut line = String::new();

        if reader
            .read_line(&mut line)
            .into_diagnostic()
            .wrap_err("Failed to read line from input stream")?
            == 0
        {
            if buf.is_empty() {
                return Ok(None);
            } else {
                return Err(miette!("EOF before end of headers"));
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
        .ok_or_else(|| miette!("Content-Length header not found"))?;

    let mut buf = buf.into_bytes();
    buf.resize(content_length, 0);
    reader
        .read_exact(&mut buf)
        .into_diagnostic()
        .wrap_err("Failed to read exact bytes from input stream")?;

    let buf = String::from_utf8(buf).map_err(|_| miette!("Failed to convert bytes to string"))?;

    Ok(Some(buf))
}

pub fn handle_message<R: BufRead>(reader: &mut R) -> Result<Message> {
    let content = read_message(reader)
        .map_err(|e| miette!(e.to_string()))?
        .ok_or_else(|| miette!("No message received"))?;

    let message: Message = serde_json::from_str(&content)
        .into_diagnostic()
        .wrap_err("Failed to parse message")?;

    Ok(message)
}
