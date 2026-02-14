// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fs::File,
    io::{self, BufWriter, Seek, Write},
    path::PathBuf,
};

pub struct SplitWriter {
    split_size: u64,
    dest_dir: PathBuf,
    get_file_name: Box<dyn Fn(usize) -> String>,
    current_pos: u64,
    writers: Vec<BufWriter<File>>,
}

impl SplitWriter {
    pub fn new(
        dest_dir: PathBuf,
        get_file_name: impl Fn(usize) -> String + 'static,
        split_size: u64,
    ) -> io::Result<SplitWriter> {
        let first_file_path = dest_dir.join(get_file_name(0));
        let first_writer = BufWriter::new(File::create(first_file_path)?);
        let writers = vec![first_writer];

        let split_writer = SplitWriter {
            split_size,
            dest_dir,
            get_file_name: Box::new(get_file_name),
            current_pos: 0,
            writers,
        };

        Ok(split_writer)
    }

    pub fn total_len(&mut self) -> io::Result<u64> {
        if self.writers.is_empty() {
            return Ok(0);
        }

        let last_i = self.writers.len() - 1;
        self.writers[last_i].flush()?;
        let last_file_len = self.writers[last_i].get_ref().metadata()?.len();
        let total_len = (last_i as u64 * self.split_size) + last_file_len;

        Ok(total_len)
    }
}

impl Write for SplitWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let file_offset = self.current_pos % self.split_size;
        let remaining_in_file = self.split_size - file_offset;

        #[allow(clippy::cast_possible_truncation)]
        let i = (self.current_pos / self.split_size) as usize;

        if i >= self.writers.len() {
            let file_name = (self.get_file_name)(i);
            let file_path = self.dest_dir.join(file_name);
            let writer = BufWriter::new(File::create(file_path)?);
            self.writers.push(writer);
        }

        let writer = &mut self.writers[i];

        #[allow(clippy::cast_possible_truncation)]
        let n_to_write = buf.len().min(remaining_in_file as usize);
        let n_written = writer.write(&buf[..n_to_write])?;
        self.current_pos += n_written as u64;

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
        self.current_pos = match pos {
            io::SeekFrom::Start(n) => n,
            io::SeekFrom::End(n) => {
                if n > 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }

                let Some(new_pos) = self.total_len()?.checked_add_signed(n) else {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                };

                new_pos
            }
            io::SeekFrom::Current(n) => {
                let Some(new_pos) = self.current_pos.checked_add_signed(n) else {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                };

                if new_pos > self.total_len()? {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }

                new_pos
            }
        };

        #[allow(clippy::cast_possible_truncation)]
        let i = (self.current_pos / self.split_size) as usize;

        let Some(writer) = self.writers.get_mut(i) else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };

        let file_offset = self.current_pos % self.split_size;
        writer.seek(io::SeekFrom::Start(file_offset))?;

        Ok(self.current_pos)
    }
}
