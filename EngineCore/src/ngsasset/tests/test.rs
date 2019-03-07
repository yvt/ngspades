//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use futures::executor::block_on;
use std::{
    collections::HashMap,
    io::{self, prelude::*},
};
use uuid::Uuid;

use ngsasset::{chunk, loader};

#[derive(Debug)]
struct MemOpenChunk(HashMap<Uuid, Box<[u8]>>);

impl loader::OpenChunk for MemOpenChunk {
    type ReadSeek = io::Cursor<Vec<u8>>;

    fn open_chunk(&self, chunk_id: Uuid) -> io::Result<Option<Self::ReadSeek>> {
        Ok(self
            .0
            .get(&chunk_id)
            .map(|bytes| io::Cursor::new(bytes[..].to_owned())))
    }
}

#[test]
fn manager() {
    let mut chunk_data: Vec<u8> = Vec::new();

    let blobs = [
        Uuid::parse_str("8fd8b9f8-4d22-48a2-990e-1b7fc29db5a1").unwrap(),
        Uuid::parse_str("ad1c9a13-1e33-4a48-aeec-e4468bf2fc6c").unwrap(),
        Uuid::parse_str("78ea264f-36b3-4afe-b669-5ad4be0306cc").unwrap(),
        Uuid::parse_str("f171347e-969a-4348-af46-97277606496b").unwrap(),
    ];

    {
        let mut chunk_writer =
            chunk::ChunkWriter::new(io::Cursor::new(&mut chunk_data), blobs.iter().cloned())
                .unwrap();
        for &blob_id in blobs.iter() {
            let mut blob_writer = chunk_writer.write_blob(blob_id).unwrap();
            blob_writer.write_all(&blob_id.as_bytes()[..]).unwrap();
            blob_writer.finish().unwrap();
        }
        chunk_writer.finish().unwrap();
    }

    println!("Chunk = {:?}", &chunk_data);

    // Create a `Manager` instance
    let chunk_id = Uuid::parse_str("8c94ffc3-707f-4756-9e72-a2a28c7b1acc").unwrap();
    let mem_chunk_fs = MemOpenChunk(Some((chunk_id, chunk_data.into())).into_iter().collect());
    let chunk_store = loader::StdChunkStore::new(mem_chunk_fs);

    let blob_index = loader::MemBlobIndex::new(blobs.iter().map(|&blob| (blob, chunk_id)));

    let mut manager = loader::Manager::new(blob_index, chunk_store);

    // Load a blob from the `Manager`
    assert_eq!(
        &block_on(manager.get_blob_mut(blobs[0])).unwrap().unwrap()[..],
        &blobs[0].as_bytes()[..]
    );

    let alio = Uuid::parse_str("31d0516d-6d77-417c-b9ed-05ee4502da4e").unwrap();
    assert!(&block_on(manager.get_blob_mut(alio)).unwrap().is_none());
}
