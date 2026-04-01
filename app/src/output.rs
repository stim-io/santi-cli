use std::io::{self, Read, Write};

use anyhow::Result;
use serde::Serialize;

pub fn read_message(message: Option<String>) -> Result<String> {
    match message {
        Some(message) => Ok(message),
        None => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}

pub fn json<T: Serialize>(value: &T) -> Result<()> {
    serde_json::to_writer_pretty(io::stdout(), value)?;
    println!();
    Ok(())
}

pub fn stream_text(text: &str) -> Result<()> {
    print!("{text}");
    io::stdout().flush()?;
    Ok(())
}
