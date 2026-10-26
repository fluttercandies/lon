//! Command-line LON codec: `lon encode [json-file]`, `lon decode [lon-file]`,
//! and `lon check [lon-file]`, reading standard input when no file is given.

use std::fs::File;
use std::io::{Read, Write};
use std::process::ExitCode;

use lon::{LonCodec, LonError};

/// Streamed input cap in bytes, mirroring the Dart CLI.
const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    let mut pretty = false;
    let mut positional = Vec::new();
    let mut valid = true;
    for arg in std::env::args().skip(1) {
        if arg == "--pretty" {
            pretty = true;
        } else if arg.starts_with('-') {
            valid = false;
        } else {
            positional.push(arg);
        }
    }
    let Some(command) = positional.first() else {
        usage();
        return ExitCode::from(64);
    };
    if !valid
        || positional.len() > 2
        || (command != "encode" && command != "decode" && command != "check")
    {
        usage();
        return ExitCode::from(64);
    }

    let source = match read_source(positional.get(1).map(String::as_str)) {
        Ok(source) => source,
        Err(InputError::Io(error)) => {
            eprintln!("{error}");
            return ExitCode::from(66);
        }
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(65);
        }
    };

    let codec = LonCodec::new().pretty(pretty);
    if command == "check" {
        // Validation only: a fully decoded document exits 0 with no output.
        return match codec.lon_to_json(&source) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                report_invalid(&error);
                ExitCode::from(65)
            }
        };
    }
    let result = if command == "encode" {
        codec.json_to_lon(&source)
    } else {
        codec.lon_to_json(&source)
    };
    match result {
        Ok(output) => write_output(&output),
        Err(error) => {
            report_invalid(&error);
            ExitCode::from(65)
        }
    }
}

enum InputError {
    Io(std::io::Error),
    TooLarge,
    InvalidUtf8,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The OS message never echoes file contents.
            InputError::Io(error) => write!(f, "{error}"),
            InputError::TooLarge => {
                write!(
                    f,
                    "Invalid input: input byte limit of {MAX_INPUT_BYTES} exceeded"
                )
            }
            InputError::InvalidUtf8 => write!(f, "Invalid input: input is not valid UTF-8"),
        }
    }
}

fn read_source(path: Option<&str>) -> Result<String, InputError> {
    let mut bytes = Vec::new();
    match path {
        Some(path) => {
            let file = File::open(path).map_err(InputError::Io)?;
            file.take(MAX_INPUT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(InputError::Io)?;
        }
        None => {
            std::io::stdin()
                .lock()
                .take(MAX_INPUT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(InputError::Io)?;
        }
    }
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(InputError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| InputError::InvalidUtf8)
}

/// Diagnostics never echo untrusted source text, only the offset and reason.
fn report_invalid(error: &LonError) {
    match error.offset() {
        Some(offset) => eprintln!("Invalid input at offset {offset}: {}", error.message()),
        None => eprintln!("Invalid input: {}", error.message()),
    }
}

fn write_output(output: &str) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    if writeln!(stdout, "{output}")
        .and_then(|()| stdout.flush())
        .is_err()
    {
        eprintln!("Failed to write output");
        return ExitCode::from(66);
    }
    ExitCode::SUCCESS
}

fn usage() {
    eprintln!(
        "Usage: lon encode [--pretty] [json-file] | lon decode [--pretty] [lon-file] | lon check [lon-file]"
    );
}
