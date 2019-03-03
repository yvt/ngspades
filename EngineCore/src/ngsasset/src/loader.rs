//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The asset loader of the Nightingales engine.
use owning_ref::{OwningRef, StableAddress};
use parking_lot::Mutex;
use std::{
    borrow::Borrow,
    collections::HashMap,
    io::{self, prelude::*},
    ops::{Deref, Range},
    sync::{mpsc, Arc, Weak},
};
use uuid::Uuid;

use crate::{
    chunk::{chunk_reader, ChunkHdrReader},
    utils::ReadWindow,
};

/// The asset loader.
///
/// `BI: `[`BlobIndex`] is used by [`Manager::get_blob`] to find the chunk
/// where a specified blob is located.
///
/// `Manager` maintains a hash map of chunks and blobs which are currently
/// loaded on memory. If a blob is already on the hash map, `get_blob` directly
/// access the containing chunk without consulting `BlobIndex`. This behavior
/// can be utilized to load blobs which are not included in `BlobIndex` by
/// loading the chunk first via [`Manager::get_chunk`].
#[derive(Debug)]
pub struct Manager<BI: BlobIndex, CS: ChunkStore> {
    blob_index: BI,
    chunk_store: CS,
    chunks: HashMap<Uuid, Weak<ManagerChunk<CS::ChunkHdr>>>,
    /// The mapping from loaded blob IDs to lochunk IDs.
    blobs: HashMap<Uuid, Uuid>,
    recv_retired_chunk: Mutex<mpsc::Receiver<(Uuid, CS::ChunkHdr)>>,
    send_retired_chunk: Mutex<mpsc::Sender<(Uuid, CS::ChunkHdr)>>,
}

#[derive(Debug)]
struct ManagerChunk<CH> {
    id: Uuid,
    chunk_data: Box<[u8]>,
    chunk_hdr: Option<CH>,
    send_retired_chunk: Mutex<mpsc::Sender<(Uuid, CH)>>,
}

/// A reference to a blob.
///
/// This includes an `Arc` to the containing chunk data. The chunk is released
/// when all references to the chunk are removed.
pub type Blob = OwningRef<Arc<dyn Deref<Target = [u8]>>, [u8]>;

/// An opaque reference to a chunk.
#[derive(Clone)]
pub struct Chunk(Arc<dyn Deref<Target = [u8]>>);

impl<BI: BlobIndex, CS: ChunkStore> Manager<BI, CS>
where
    CS::ChunkHdr: 'static,
{
    /// Construct a `Manager`.
    pub fn new(blob_index: BI, chunk_store: CS) -> Self {
        let (send, recv) = mpsc::channel();
        Self {
            blob_index,
            chunk_store,
            chunks: HashMap::new(),
            blobs: HashMap::new(),
            recv_retired_chunk: Mutex::new(recv),
            send_retired_chunk: Mutex::new(send),
        }
    }

    /// Get a reference to the inner `BlobIndex`.
    pub fn blob_index(&self) -> &BI {
        &self.blob_index
    }

    /// Get a mutable reference to the inner `BlobIndex`.
    pub fn blob_index_mut(&mut self) -> &mut BI {
        &mut self.blob_index
    }

    /// Get a reference to the inner `ChunkStore`.
    pub fn chunk_store(&self) -> &CS {
        &self.chunk_store
    }

    /// Get a mutable reference to the inner `ChunkStore`.
    pub fn chunk_store_mut(&mut self) -> &mut CS {
        &mut self.chunk_store
    }

    /// Get the inner `BlobIndex` and `ChunkStore`, consuming `self`.
    pub fn into_inner(self) -> (BI, CS) {
        (self.blob_index, self.chunk_store)
    }

    /// Delete the cache entries for unloaded chunks.
    pub fn purge(&mut self) -> io::Result<()> {
        for (chunk_id, chunk_hdr) in self.recv_retired_chunk.get_mut().try_iter() {
            let ok = self.chunks.get(&chunk_id).and_then(Weak::upgrade).is_none();
            if !ok {
                // The chunk is still is use
                continue;
            }

            self.chunks.remove(&chunk_id);

            for blob_id in chunk_hdr
                .get_blobs()
                .map_err(|x| -> io::Error { x.into() })?
            {
                if self.blobs.get(&blob_id) != Some(&chunk_id) {
                    continue;
                }
                self.blobs.remove(&blob_id);
            }
        }

        Ok(())
    }

    /// Load a blob.
    pub fn get_blob(&mut self, blob_id: Uuid) -> Result<Option<Blob>, ManagerError> {
        self.purge()?;

        // Check the cache
        let blob = self
            .blobs
            .get(&blob_id)
            .and_then(|chunk_id: &Uuid| self.chunks.get(chunk_id))
            .and_then(Weak::upgrade)
            .and_then(|chunk_arc: Arc<ManagerChunk<_>>| {
                match chunk_arc.chunk_hdr().get_blob_range(blob_id) {
                    Ok(Some(range)) => Some(Ok((range, chunk_arc))),
                    Ok(None) => None,
                    Err(x) => Some(Err(x)),
                }
            })
            .transpose()?
            .map(|(range, chunk_arc)| ManagerChunk::slice_blob(chunk_arc, range));

        if let Some(blob) = blob {
            return Ok(Some(blob));
        }

        // Locate the chunk where the blob is located
        let chunk_id = if let Some(x) = self.blob_index.chunk_for_blob(blob_id)? {
            x
        } else {
            return Ok(None);
        };

        // Load the chunk is not loaded yet
        let chunk_arc = self
            .get_chunk_arc(chunk_id)?
            .ok_or(ManagerError::MissingChunk { blob_id, chunk_id })?;

        // Find the blob within the chunk
        let range = match chunk_arc.chunk_hdr().get_blob_range(blob_id) {
            Ok(Some(range)) => range,
            Ok(None) => return Err(ManagerError::BadBlobIndex { blob_id, chunk_id }),
            Err(x) => return Err(x.into()),
        };
        Ok(Some(ManagerChunk::slice_blob(chunk_arc, range)))
    }

    /// Load a chunk and get an opaque refernce to it.
    pub fn get_chunk(&mut self, chunk_id: Uuid) -> io::Result<Option<Chunk>> {
        Ok(self
            .get_chunk_arc(chunk_id)?
            .map(|chunk_arc| Chunk(chunk_arc)))
    }

    fn get_chunk_arc(
        &mut self,
        chunk_id: Uuid,
    ) -> io::Result<Option<Arc<ManagerChunk<CS::ChunkHdr>>>> {
        let chunk_weak = self.chunks.entry(chunk_id).or_default();
        let chunk_arc = chunk_weak.upgrade();

        if let Some(x) = chunk_arc {
            return Ok(Some(x));
        }

        // Load the chunk
        let (chunk_hdr, mut chunk_stream) =
            if let Some(x) = self.chunk_store.get_chunk_stream(chunk_id)? {
                x
            } else {
                return Ok(None);
            };

        let mut chunk_data = Vec::new();
        chunk_stream.read_to_end(&mut chunk_data)?;

        let send_retired_chunk = Mutex::new(mpsc::Sender::clone(self.send_retired_chunk.get_mut()));

        let chunk = ManagerChunk {
            id: chunk_id,
            chunk_hdr: Some(chunk_hdr),
            chunk_data: chunk_data.into(),
            send_retired_chunk,
        };

        let chunk_arc = Arc::new(chunk);
        *chunk_weak = Arc::downgrade(&chunk_arc);

        // Update the cache
        for blob_id in chunk_arc.chunk_hdr().get_blobs()? {
            self.blobs.insert(blob_id, chunk_id);
        }

        Ok(Some(chunk_arc))
    }
}

impl<CH: 'static> ManagerChunk<CH> {
    fn slice_blob(this: Arc<Self>, range: Range<u64>) -> Blob {
        let range = range.start as usize..range.end as usize;
        OwningRef::new(this as Arc<dyn Deref<Target = [u8]>>).map(move |this| &this[range.clone()])
    }

    fn chunk_hdr(&self) -> &CH {
        self.chunk_hdr.as_ref().unwrap()
    }
}

impl<CH> Drop for ManagerChunk<CH> {
    fn drop(&mut self) {
        // Request to delete the cache of this chunk from `Manager`
        let chunk_hdr = self.chunk_hdr.take().unwrap();
        let payload = (self.id, chunk_hdr);

        // Ignore send error
        drop(self.send_retired_chunk.get_mut().send(payload));
    }
}

// Enable unsizing `Arc<ManagerChunk<CH>>` into `Arc<dyn ...>`.
// We do not directly use this deref
impl<CH> Deref for ManagerChunk<CH> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.chunk_data
    }
}

// This is safe because this type does not have an interior mutability.
unsafe impl<CH> StableAddress for ManagerChunk<CH> {}

/// Locates a chunk containing the blob with a given UUID.
pub trait BlobIndex {
    /// Locate a chunk containing the blob with a given UUID.
    fn chunk_for_blob(&self, blob_id: Uuid) -> io::Result<Option<Uuid>>;
}

/// An empty `BlobIndex`.
#[derive(Debug, Clone, Copy)]
pub struct NoBlobIndex;

impl BlobIndex for NoBlobIndex {
    fn chunk_for_blob(&self, _blob_id: Uuid) -> io::Result<Option<Uuid>> {
        Ok(None)
    }
}

/// An in-memory implementation of `BlobIndex`.
#[derive(Debug, Clone)]
pub struct MemBlobIndex(HashMap<Uuid, Uuid>);

impl MemBlobIndex {
    /// Construct a `MemBlobIndex` from an iterator of `(blob ID, chunk ID)`.
    pub fn new(blob_chunk: impl Iterator<Item = (Uuid, Uuid)>) -> Self {
        Self(blob_chunk.collect())
    }
}

impl BlobIndex for MemBlobIndex {
    fn chunk_for_blob(&self, blob_id: Uuid) -> io::Result<Option<Uuid>> {
        Ok(self.0.get(&blob_id).cloned())
    }
}

/// The error type for [`Manager`].
#[derive(Debug)]
pub enum ManagerError {
    /// [`BlobIndex`] indicated that `blob_id` can be found in `chunk_id`,
    /// `chunk_id` was not found.
    MissingChunk {
        blob_id: Uuid,
        chunk_id: Uuid,
    },
    /// [`BlobIndex`] indicated that `blob_id` can be found in `chunk_id`, but
    /// it actually could not be found there.
    BadBlobIndex {
        blob_id: Uuid,
        chunk_id: Uuid,
    },
    IoError(io::Error),
}

impl std::error::Error for ManagerError {}

impl std::fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ManagerError::MissingChunk { blob_id, chunk_id } => write!(
                f,
                "The blob {} was indicated to be in the chunk {}, which doesn't exist",
                blob_id, chunk_id
            ),
            ManagerError::BadBlobIndex { blob_id, chunk_id } => write!(
                f,
                "The blob {} was indicated to be in the chunk {}, but acutally it wasn't",
                blob_id, chunk_id
            ),
            ManagerError::IoError(x) => std::fmt::Display::fmt(x, f),
        }
    }
}

impl From<io::Error> for ManagerError {
    fn from(x: io::Error) -> Self {
        ManagerError::IoError(x)
    }
}

impl From<ManagerError> for io::Error {
    fn from(x: ManagerError) -> Self {
        match x {
            ManagerError::IoError(x) => x,
            x => io::Error::new(io::ErrorKind::Other, x),
        }
    }
}

/// Loads chunks upon request.
pub trait ChunkStore {
    type ChunkHdr: ChunkHdr;

    /// A stream for reading chunk data.
    type ChunkRead: ReadSeek;

    /// Open a chunk for streamed read.
    fn get_chunk_stream(
        &mut self,
        chunk_id: Uuid,
    ) -> io::Result<Option<(Self::ChunkHdr, Self::ChunkRead)>>;
}

/// A helper trait for simulating `dyn Read + Seek`.
pub trait ReadSeek: Read + Seek {}

impl<T: ?Sized + Read + Seek> ReadSeek for T {}

/// Provides the information about the layout of blobs within a chunk.
///
/// [`ChunkHdrReader`](crate::chunk::ChunkHdrReader) is an implementation of
/// this trait for chunks in the standard chunk format.
pub trait ChunkHdr {
    /// Find the position of the blob data within [`ChunkStore::ChunkRead`] for
    /// a given blob ID.
    fn get_blob_range(&self, blob_id: Uuid) -> io::Result<Option<Range<u64>>>;

    /// Enumerate all blobs included in the chunk.
    fn get_blobs<'a>(&'a self) -> io::Result<Box<dyn Iterator<Item = Uuid> + 'a>>;
}

impl<T: Borrow<[u8]>> ChunkHdr for ChunkHdrReader<T> {
    fn get_blob_range(&self, blob_id: Uuid) -> io::Result<Option<Range<u64>>> {
        Ok(self.get_blob_range(blob_id)?)
    }
    fn get_blobs<'a>(&'a self) -> io::Result<Box<dyn Iterator<Item = Uuid> + 'a>> {
        Ok(self.get_blobs().map(|it| Box::new(it) as _)?)
    }
}

/// Implements [`ChunkStore`] by using [`OpenChunk`] as a backend for opening
/// a chunk stream and interpreting the data as the standard chunk format.
#[derive(Debug)]
pub struct StdChunkStore<OC> {
    open_chunk: OC,
}

impl<OC> StdChunkStore<OC> {
    /// Construct a `StdChunkStore` with a `OpenChunk` used to open chunk
    /// streams.
    pub fn new(open_chunk: OC) -> Self {
        Self { open_chunk }
    }

    /// Get a reference to the inner `OpenChunk`.
    pub fn open_chunk(&self) -> &OC {
        &self.open_chunk
    }

    /// Get a mutable reference to the inner `OpenChunk`.
    pub fn open_chunk_mut(&mut self) -> &mut OC {
        &mut self.open_chunk
    }

    /// Get the inner `OpenChunk`, consuming `self`.
    pub fn into_open_chunk(self) -> OC {
        self.open_chunk
    }
}

impl<OC: OpenChunk> ChunkStore for StdChunkStore<OC> {
    type ChunkHdr = ChunkHdrReader<Box<[u8]>>;
    type ChunkRead = ReadWindow<OC::ReadSeek>;

    fn get_chunk_stream(
        &mut self,
        chunk_id: Uuid,
    ) -> io::Result<Option<(Self::ChunkHdr, Self::ChunkRead)>> {
        let stream = if let Some(x) = self.open_chunk.open_chunk(chunk_id)? {
            x
        } else {
            return Ok(None);
        };

        let (data_reader, hdr) = chunk_reader(stream)?;

        // We do not use `ChunkDataReader` here. We just read the rest of the
        // chunk into a buffer instead
        let mut data_reader = data_reader.into_inner();
        let start = data_reader.seek(io::SeekFrom::Current(0))?;
        let end = data_reader.seek(io::SeekFrom::End(0))?;
        let windowed_data_reader = ReadWindow::new(data_reader, start..end)?;

        let hdr = ChunkHdrReader(hdr.into());

        Ok(Some((hdr, windowed_data_reader)))
    }
}

/// The trait used by `StdChunkStore` to open a raw byte stream of the standard
/// chunk format.
pub trait OpenChunk {
    type ReadSeek: ReadSeek;
    fn open_chunk(&mut self, chunk_id: Uuid) -> io::Result<Option<Self::ReadSeek>>;
}
