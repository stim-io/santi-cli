use std::io::{self, IsTerminal, Read, Write};

use anyhow::Result;
use serde::Serialize;

pub fn read_message(message: Option<String>) -> Result<String> {
    read_message_from(message, &mut io::stdin(), io::stdin().is_terminal())
}

fn read_message_from<R: Read>(
    message: Option<String>,
    input: &mut R,
    stdin_is_terminal: bool,
) -> Result<String> {
    match message {
        Some(message) => Ok(message),
        None if stdin_is_terminal => Ok(String::new()),
        None => {
            let mut input_text = String::new();
            input.read_to_string(&mut input_text)?;
            Ok(input_text)
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

pub fn stderr_line(text: &str) -> Result<()> {
    eprintln!("{text}");
    io::stderr().flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::read_message_from;

    #[test]
    fn returns_empty_string_without_blocking_for_interactive_stdin() {
        let mut input = Cursor::new("ignored");

        let result = read_message_from(None, &mut input, true).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn still_reads_piped_stdin_when_no_message_is_provided() {
        let mut input = Cursor::new("hello from stdin");

        let result = read_message_from(None, &mut input, false).unwrap();

        assert_eq!(result, "hello from stdin");
    }
}
