// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
use size::Size;

#[cfg(feature = "cli")]
const USAGE: &str = "Usage: split-write [-s|--split-size SIZE] SOURCE DESTDIR";

#[cfg(feature = "cli")]
struct Args {
    split_size: Size,
    source: PathBuf,
    dest_dir: PathBuf,
}

#[cfg(feature = "cli")]
fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut split_size = Size::from_bytes(0);
    let mut source = PathBuf::new();
    let mut dest_dir = PathBuf::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Value(val) => {
                if source.as_os_str().is_empty() {
                    source = PathBuf::from(val);
                } else {
                    dest_dir = PathBuf::from(val);
                }
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

    assert!(!source.as_os_str().is_empty(), "{USAGE}");

    Ok(Args {
        split_size,
        source,
        dest_dir,
    })
}

#[cfg(feature = "cli")]
fn main() -> Result<(), lexopt::Error> {
    use split_write::SplitWriter;
    use std::{ffi::OsStr, fs::File, io::BufReader};

    let args = parse_args()?;

    assert!(args.source.is_file(), "Source path must be a file");
    assert!(args.dest_dir.is_dir(), "Dest path must be a directory");

    let Some(file_stem) = args
        .source
        .file_stem()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
    else {
        panic!("Source path must have a file stem");
    };

    let Some(file_ext) = args
        .source
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
    else {
        panic!("Source path must have an extension");
    };

    let Ok(split_size) = args.split_size.bytes().try_into() else {
        panic!("Invalid split size");
    };

    let get_file_name = move |n| format!("{file_stem}.part{n}.{file_ext}");

    let source_file = File::open(&args.source).expect("Failed to open source file");
    let mut reader = BufReader::new(source_file);

    let mut writer = SplitWriter::new(args.dest_dir, get_file_name, split_size)
        .expect("Failed to create split writer");

    std::io::copy(&mut reader, &mut writer).expect("Failed to copy file");

    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("Please add the `cli` feature to enable the CLI");
}
