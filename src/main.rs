// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
use size::Size;

#[cfg(feature = "cli")]
const USAGE: &str = "Usage: split-write [-s|--split-size SIZE] DEST_DIR < SOURCE_FILE";

#[cfg(feature = "cli")]
struct Args {
    split_size: Size,
    dest_dir: PathBuf,
}

#[cfg(feature = "cli")]
fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut split_size = Size::from_bytes(0);
    let mut dest_dir = PathBuf::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Value(val) => {
                dest_dir = PathBuf::from(val);
            }
            Short('s') | Long("split-size") => {
                let size_str = parser.value()?.to_string_lossy().to_string();
                split_size = Size::from_str(size_str.as_str()).expect("Failed to parse split size");
            }
            Short('h') | Long("help") => {
                eprintln!("{USAGE}");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        split_size,
        dest_dir,
    })
}

#[cfg(feature = "cli")]
fn main() -> Result<(), lexopt::Error> {
    let args = parse_args()?;

    assert!(args.dest_dir.is_dir(), "Dest path must be a directory");

    let Ok(split_size) = u64::try_from(args.split_size.bytes()) else {
        panic!("Invalid split size");
    };

    let get_file_name = |n| format!("random.part{n}.bin");

    let mut reader = std::io::stdin();

    let mut writer = split_write::SplitWriter::new(args.dest_dir, get_file_name, split_size)
        .expect("Failed to create split writer");

    std::io::copy(&mut reader, &mut writer).expect("Failed to copy file");

    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("Please add the `cli` feature to enable the CLI");
}
