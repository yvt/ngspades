//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use phf_shared::PhfHash;
use std::hash::Hasher;
use std::{
    cmp::min,
    io::{self, prelude::*},
    ops::Range,
};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HashableUuid(pub Uuid);

impl PhfHash for HashableUuid {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().phf_hash(state);
    }
}

#[derive(Debug)]
pub struct ReadWindow<T> {
    reader: T,
    offset: u64,
    len: u64,
    cursor: u64,
}

impl<T: Read + Seek> ReadWindow<T> {
    pub fn new(mut reader: T, range: Range<u64>) -> io::Result<Self> {
        reader.seek(io::SeekFrom::Start(range.start))?;
        Ok(Self {
            reader,
            offset: range.start,
            len: range.end - range.start,
            cursor: 0,
        })
    }
}

impl<T: Read + Seek> Read for ReadWindow<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = min(self.len.saturating_sub(self.cursor), buf.len() as u64);
        let buf = &mut buf[0..left as usize];
        let bytes_read = self.reader.read(buf)?;
        self.cursor += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl<T: Read + Seek> Seek for ReadWindow<T> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let pos = match pos {
            io::SeekFrom::Start(x) => x,
            io::SeekFrom::End(x) => (self.len as i64 + x) as u64,
            io::SeekFrom::Current(x) => (self.offset as i64 + x) as u64,
        };
        self.reader.seek(io::SeekFrom::Start(self.offset + pos))?;
        self.cursor = pos;
        Ok(pos)
    }
}
