// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fs::File,
    io::{self, BufWriter, Seek, Write},
    num::NonZeroU64,
    path::PathBuf,
};

pub struct SplitWriter<F> {
    split_size: NonZeroU64,
    dest_dir: PathBuf,
    get_file_name: F,
    current_pos: u64,
    total_len: u64,
    writers: Vec<BufWriter<File>>,
}

impl<F> SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    pub fn new(dest_dir: PathBuf, get_file_name: F, split_size: NonZeroU64) -> Self {
        Self {
            split_size,
            dest_dir,
            get_file_name,
            current_pos: 0,
            total_len: 0,
            writers: Vec::new(),
        }
    }

    pub fn file_count(&self) -> usize {
        self.writers.len()
    }
}

impl<F> Write for SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let i = usize::try_from(self.current_pos / self.split_size.get())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;

        // Safely fill gaps if the user seeks far ahead
        while self.writers.len() <= i {
            let idx = self.writers.len();
            let file_name = (self.get_file_name)(idx);
            let file_path = self.dest_dir.join(file_name);
            let file = File::create(file_path)?;

            self.writers.push(BufWriter::with_capacity(32_768, file));
        }

        let writer = &mut self.writers[i];

        // Ensure the underlying writer is physically at the correct offset
        // before writing, in case we just jumped here via Seek.
        let file_offset = self.current_pos % self.split_size.get();
        writer.seek(io::SeekFrom::Start(file_offset))?;

        let remaining_in_file =
            usize::try_from(self.split_size.get() - file_offset).unwrap_or(usize::MAX);

        let n_to_write = buf.len().min(remaining_in_file);
        let n_written = writer.write(&buf[..n_to_write])?;

        self.current_pos += n_written as u64;
        self.total_len = self.total_len.max(self.current_pos);

        Ok(n_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        for w in &mut self.writers {
            w.flush()?;
        }
        Ok(())
    }
}

impl<F> Seek for SplitWriter<F>
where
    F: Fn(usize) -> String,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.current_pos = match pos {
            io::SeekFrom::Start(n) => n,
            io::SeekFrom::End(n) => self
                .total_len
                .checked_add_signed(n)
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?,
            io::SeekFrom::Current(n) => self
                .current_pos
                .checked_add_signed(n)
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?,
        };

        Ok(self.current_pos)
    }
}
