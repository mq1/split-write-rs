// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fs::File,
    io::{self, Seek, Write},
    num::NonZeroU64,
    path::PathBuf,
};

#[derive(Debug)]
pub struct SplitWriter<F> {
    split_size: NonZeroU64,
    dest_dir: PathBuf,
    get_file_name: F,
    current_pos: u64,
    first_file: File,
    last_file: Option<File>,
    file_count: usize,
}

impl<F> SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    pub fn try_new(
        dest_dir: impl Into<PathBuf>,
        get_file_name: F,
        split_size: NonZeroU64,
    ) -> io::Result<Self> {
        let dest_dir = dest_dir.into();
        let first_file = File::create(dest_dir.join(get_file_name(0)))?;

        Ok(Self {
            split_size,
            dest_dir,
            get_file_name,
            current_pos: 0,
            first_file,
            last_file: None,
            file_count: 1,
        })
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }

    pub fn total_size(&self) -> u64 {
        self.current_pos
    }

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

        #[allow(clippy::cast_possible_truncation)]
        let i = (self.current_pos / self.split_size.get()) as usize;

        if self.file_count <= i {
            let idx = self.file_count;
            let file_name = (self.get_file_name)(idx);
            let file_path = self.dest_dir.join(file_name);
            let file = File::create(file_path)?;

            if let Some(last_file) = &mut self.last_file {
                last_file.flush()?;
            }

            self.last_file = Some(file);
            self.file_count += 1;
        }

        let (file, offset) = if let Some(last_file) = &mut self.last_file {
            (last_file, self.current_pos % self.split_size.get())
        } else {
            (&mut self.first_file, self.current_pos)
        };

        let to_write = match usize::try_from(self.split_size.get() - offset) {
            Ok(remaining) => buf.len().min(remaining),
            Err(_) => buf.len(),
        };

        let n = file.write(&buf[..to_write])?;

        self.current_pos += n as u64;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(last_file) = &mut self.last_file {
            last_file.flush()?;
        }

        self.first_file.flush()
    }
}
