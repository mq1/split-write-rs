// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fs::File,
    io::{self, BufWriter, Seek, Write},
    num::NonZeroU64,
    path::PathBuf,
};

pub struct SplitWriter {
    split_size: NonZeroU64,
    dest_dir: PathBuf,
    get_file_name: Box<dyn Fn(usize) -> String + Send + Sync>,
    current_pos: u64,
    total_len: u64,
    writers: Vec<BufWriter<File>>,
}

impl SplitWriter {
    pub fn new(
        dest_dir: PathBuf,
        get_file_name: impl Fn(usize) -> String + Send + Sync + 'static,
        split_size: NonZeroU64,
    ) -> io::Result<SplitWriter> {
        let first_file_path = dest_dir.join(get_file_name(0));
        let first_writer = BufWriter::with_capacity(32_768, File::create(first_file_path)?);
        let writers = vec![first_writer];

        Ok(SplitWriter {
            split_size,
            dest_dir,
            get_file_name: Box::new(get_file_name),
            current_pos: 0,
            total_len: 0,
            writers,
        })
    }
}

impl Write for SplitWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Ok(i) = usize::try_from(self.current_pos / self.split_size.get()) else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };

        if i >= self.writers.len() {
            let file_name = (self.get_file_name)(i);
            let file_path = self.dest_dir.join(file_name);
            let writer = BufWriter::with_capacity(32_768, File::create(file_path)?);
            self.writers.push(writer);
        }

        let writer = &mut self.writers[i];

        let file_offset = self.current_pos % self.split_size.get();
        let Ok(remaining_in_file) = usize::try_from(self.split_size.get() - file_offset) else {
            return Err(io::Error::from(io::ErrorKind::FileTooLarge));
        };

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

impl Seek for SplitWriter {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            io::SeekFrom::Start(n) => n,
            io::SeekFrom::End(n) => {
                let Some(new_pos) = self.total_len.checked_add_signed(n) else {
                    return Err(io::Error::from(io::ErrorKind::InvalidInput));
                };

                new_pos
            }
            io::SeekFrom::Current(n) => {
                let Some(new_pos) = self.current_pos.checked_add_signed(n) else {
                    return Err(io::Error::from(io::ErrorKind::InvalidInput));
                };

                new_pos
            }
        };

        if new_pos > self.total_len {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }

        self.current_pos = new_pos;

        let Ok(i) = usize::try_from(self.current_pos / self.split_size.get()) else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };

        if i > self.writers.len() {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }

        if i < self.writers.len() {
            let file_offset = self.current_pos % self.split_size.get();
            self.writers[i].seek(io::SeekFrom::Start(file_offset))?;

            for next_writer in &mut self.writers[i + 1..] {
                next_writer.rewind()?;
            }
        }

        Ok(self.current_pos)
    }
}
