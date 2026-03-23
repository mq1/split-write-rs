// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fs::File,
    io::{self, Seek, Write},
    num::NonZeroUsize,
    path::PathBuf,
};

#[derive(Debug)]
pub struct SplitWriter<F> {
    split_size: Option<NonZeroUsize>,
    dest_dir: PathBuf,
    get_file_name: F,
    current_offset: usize,
    first_file: File,
    last_file: Option<File>,
    current_i: usize,
    total_size: u64,
}

impl<F> SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    pub fn create(
        dest_dir: impl Into<PathBuf>,
        get_file_name: F,
        split_size: Option<NonZeroUsize>,
    ) -> io::Result<Self> {
        let dest_dir = dest_dir.into();
        let first_file = File::create(dest_dir.join(get_file_name(0)))?;

        Ok(Self {
            split_size,
            dest_dir,
            get_file_name,
            current_offset: 0,
            first_file,
            last_file: None,
            current_i: 0,
            total_size: 0,
        })
    }

    pub fn file_count(&self) -> usize {
        self.current_i + 1
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Don't do any more writes after calling this!
    pub fn write_header(&mut self, header: &[u8]) -> io::Result<()> {
        self.first_file.rewind()?;
        self.first_file.write_all(header)
    }
}

impl<F> Write for SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self
            .split_size
            .is_some_and(|s| s.get() == self.current_offset)
        {
            self.current_i += 1;

            let file_name = (self.get_file_name)(self.current_i);
            let file_path = self.dest_dir.join(file_name);
            let file = File::create(file_path)?;

            self.last_file = Some(file);
            self.current_offset = 0;
        }

        let current_file = self.last_file.as_mut().unwrap_or(&mut self.first_file);

        let written = if let Some(split_size) = self.split_size {
            let remaining = split_size.get() - self.current_offset;
            let to_write = buf.len().min(remaining);
            let written = current_file.write(&buf[..to_write])?;
            self.current_offset += written;
            written
        } else {
            current_file.write(buf)?
        };

        self.total_size += written as u64;

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(last_file) = &mut self.last_file {
            last_file.flush()?;
        }

        self.first_file.flush()
    }
}
