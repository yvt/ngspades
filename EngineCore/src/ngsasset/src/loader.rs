//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The asset loader of the Nightingales engine.
use arc_io_error::IoError;
use atom2::SetOnceAtom;
use futures::{future, prelude::*};
use lockable::BorrowLock;
use multicastfuture::MultiCast;
use owning_ref::{Erased, ErasedArcRef, OwningRef};
use parking_lot::Mutex;
use std::{
    borrow::Borrow,
    collections::HashMap,
    io::{self, prelude::*},
    ops::Range,
    pin::Pin,
    sync::{mpsc, Arc, Weak},
};
use uuid::Uuid;

use crate::{
    chunk::{async_chunk_reader, ChunkHdrReader},
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
pub struct Manager<BI: BlobIndex, CS: ChunkStore> {
    blob_index: Pin<Arc<BI>>,
    chunk_store: Pin<Arc<CS>>,
    chunks: HashMap<Uuid, Weak<ManagerChunk<CS::ChunkHdr>>>,
    /// The mapping from loaded blob IDs to chunk IDs.
    blobs: HashMap<Uuid, Uuid>,
    recv_retired_chunk: Mutex<mpsc::Receiver<RetirePacket<CS::ChunkHdr>>>,
    send_retired_chunk: Mutex<mpsc::Sender<RetirePacket<CS::ChunkHdr>>>,
}

struct ManagerChunk<CH> {
    id: Uuid,
    loader: Pin<
        Arc<MultiCast<dyn Future<Output = Result<Option<Arc<ManagerChunkData<CH>>>, IoError>>>>,
    >,
    data: SetOnceAtom<Arc<ManagerChunkData<CH>>>,
    send_retired_chunk: Mutex<mpsc::Sender<RetirePacket<CH>>>,
}

type RetirePacket<CH> = (Uuid, Option<Arc<ManagerChunkData<CH>>>);

#[derive(Debug)]
struct ManagerChunkData<CH> {
    chunk_data: Box<[u8]>,
    chunk_hdr: CH,
}

/// A reference to a blob.
///
/// This includes an `Arc` to the containing chunk data. The chunk is released
/// when all references to the chunk are removed.
pub type Blob = ErasedArcRef<[u8]>;

/// An opaque reference to a chunk.
#[derive(Clone)]
pub struct Chunk(Arc<dyn Erased>);

impl<BI: BlobIndex, CS: ChunkStore> Manager<BI, CS>
where
    CS::ChunkHdr: 'static,
{
    /// Construct a `Manager`.
    pub fn new(blob_index: BI, chunk_store: CS) -> Self {
        let (send, recv) = mpsc::channel();
        Self {
            blob_index: Arc::pin(blob_index),
            chunk_store: Arc::pin(chunk_store),
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

    /// Get a reference to the inner `ChunkStore`.
    pub fn chunk_store(&self) -> &CS {
        &self.chunk_store
    }

    /// Delete the cache entries for unloaded chunks.
    pub fn purge(&mut self) -> io::Result<()> {
        for (chunk_id, chunk_data) in self.recv_retired_chunk.get_mut().try_iter() {
            let ok = self.chunks.get(&chunk_id).and_then(Weak::upgrade).is_none();
            if !ok {
                // The chunk is still is use
                continue;
            }

            self.chunks.remove(&chunk_id);

            if BI::is_fast() {
                continue;
            }

            if let Some(chunk_data) = chunk_data {
                for blob_id in chunk_data
                    .chunk_hdr
                    .get_blobs()
                    .map_err(|x| -> io::Error { x.into() })?
                {
                    if self.blobs.get(&blob_id) != Some(&chunk_id) {
                        continue;
                    }
                    self.blobs.remove(&blob_id);
                }
            }
        }

        Ok(())
    }

    /// Load a blob.
    pub fn get_blob<'a>(
        this: &'a mut impl BorrowLock<Self>,
        blob_id: Uuid,
    ) -> impl Future<Output = Result<Option<Blob>, ManagerError>> + 'a {
        async move {
            let mut this_guard = this.borrow_lock();
            this_guard.purge()?;

            // Check the cache
            let blob = this_guard
                .blobs
                .get(&blob_id)
                .and_then(|chunk_id: &Uuid| this_guard.chunks.get(chunk_id))
                .and_then(Weak::upgrade)
                .and_then(|chunk_arc: Arc<ManagerChunk<_>>| {
                    match chunk_arc.chunk_hdr().get_blob_range(blob_id) {
                        Ok(Some(range)) => Some(Ok((range, chunk_arc))),
                        Ok(None) => None,
                        Err(x) => Some(Err(x)),
                    }
                })
                .transpose()?
                .map(|(range, chunk_arc)| chunk_arc.slice_blob(range));

            if let Some(blob) = blob {
                return Ok(Some(blob));
            }

            let blob_index = Pin::clone(&this_guard.blob_index);

            drop(this_guard);

            // Locate the chunk where the blob is located
            let chunk_id = if let Some(x) = r#await!(blob_index.chunk_for_blob(blob_id))? {
                x
            } else {
                return Ok(None);
            };

            drop(blob_index);

            // Load the chunk is not loaded yet
            let chunk_arc = r#await!(Self::get_chunk_arc(this, chunk_id))?
                .ok_or(ManagerError::MissingChunk { blob_id, chunk_id })?;

            // Find the blob within the chunk
            let range = match chunk_arc.chunk_hdr().get_blob_range(blob_id) {
                Ok(Some(range)) => range,
                Ok(None) => return Err(ManagerError::BadBlobIndex { blob_id, chunk_id }),
                Err(x) => return Err(x.into()),
            };
            Ok(Some(chunk_arc.slice_blob(range)))
        }
    }

    /// A wrapper of [`Manager::get_blob`] with a `&mut Self` receiver.
    pub fn get_blob_mut<'a>(
        &'a mut self,
        blob_id: Uuid,
    ) -> impl Future<Output = Result<Option<Blob>, ManagerError>> + 'a {
        async move {
            let mut this = &mut *self;
            await!(Self::get_blob(&mut this, blob_id))
        }
    }

    /// Load a chunk and get an opaque refernce to it.
    pub fn get_chunk<'a>(
        this: &'a mut impl BorrowLock<Self>,
        chunk_id: Uuid,
    ) -> impl Future<Output = io::Result<Option<Chunk>>> + 'a {
        async move {
            let this = &mut *this;
            Ok(r#await!(Self::get_chunk_arc(this, chunk_id))?.map(|chunk_arc| Chunk(chunk_arc)))
        }
    }

    /// Load and get a chunk's `ManagerChunk`.
    ///
    /// The returned `ManagerChunk`'s `data` is guaranteed to be filled.s
    fn get_chunk_arc<'a>(
        this: &'a mut impl BorrowLock<Self>,
        chunk_id: Uuid,
    ) -> impl Future<Output = io::Result<Option<Arc<ManagerChunk<CS::ChunkHdr>>>>> + 'a {
        async move {
            let mut this_guard = this.borrow_lock();
            let this_g = &mut *this_guard; // enable split borrow

            let chunk_weak = this_g.chunks.entry(chunk_id).or_default();
            let chunk_arc = chunk_weak.upgrade();

            let chunk_arc = if let Some(x) = chunk_arc {
                x
            } else {
                // Start loading the chunk
                let send_retired_chunk =
                    Mutex::new(mpsc::Sender::clone(this_g.send_retired_chunk.get_mut()));

                let get_chunk_stream = this_g.chunk_store.get_chunk_stream(chunk_id);
                let multi_cast = Arc::pin(MultiCast::new(async {
                    match r#await!(get_chunk_stream) {
                        Ok(Some((chunk_hdr, mut chunk_stream))) => {
                            // Read the chunk data stream
                            let mut chunk_data = Vec::new();
                            await!(chunk_stream.read_to_end(&mut chunk_data))?;

                            let data = ManagerChunkData {
                                chunk_hdr,
                                chunk_data: chunk_data.into(),
                            };

                            Ok(Some(Arc::new(data)))
                        }
                        Ok(None) => Ok(None),
                        Err(x) => Err(x.into()),
                    }
                }));

                let chunk = ManagerChunk {
                    id: chunk_id,
                    send_retired_chunk,
                    loader: multi_cast,
                    data: SetOnceAtom::empty(),
                };

                let chunk_arc = Arc::new(chunk);
                *chunk_weak = Arc::downgrade(&chunk_arc);

                chunk_arc
            };

            drop(this_g);
            drop(this_guard);

            // Is the data ready?
            if chunk_arc.data.as_inner_ref().is_none() {
                // Wait until the result is ready...
                let data = match r#await!(Pin::clone(&chunk_arc.loader).subscribe()) {
                    Ok(Some(data)) => data,
                    Ok(None) => return Ok(None),
                    Err(err) => return Err(io::Error::new(err.kind(), err)),
                };

                if let Ok(()) = chunk_arc.data.store(Some(data)) {
                    // Update the cache
                    if !BI::is_fast() {
                        let mut this = this.borrow_lock();

                        for blob_id in chunk_arc.chunk_hdr().get_blobs()? {
                            this.blobs.insert(blob_id, chunk_id);
                        }
                    }
                }
            }

            Ok(Some(chunk_arc))
        }
    }
}

impl<CH: 'static> ManagerChunk<CH> {
    fn slice_blob(self: Arc<Self>, range: Range<u64>) -> Blob {
        let range = range.start as usize..range.end as usize;
        OwningRef::new(self)
            .map(move |this| &this.data.as_inner_ref().unwrap().chunk_data[range.clone()])
            .erase_owner()
    }

    fn chunk_hdr(&self) -> &CH {
        &self.data.as_inner_ref().unwrap().chunk_hdr
    }
}

impl<CH> Drop for ManagerChunk<CH> {
    fn drop(&mut self) {
        // Request to delete the cache of this chunk from `Manager`
        let payload = (self.id, self.data.take());

        // Ignore send error
        drop(self.send_retired_chunk.get_mut().send(payload));
    }
}

/// Locates a chunk containing the blob with a given UUID.
pub trait BlobIndex {
    type ChunkForBlob: Future<Output = io::Result<Option<Uuid>>> + 'static;

    /// Locate a chunk containing the blob with a given UUID.
    fn chunk_for_blob(self: &Pin<Arc<Self>>, blob_id: Uuid) -> Self::ChunkForBlob;

    /// Returns `true` if `Self` is memory-based.
    ///
    /// [`Manager`] does not create an in-memory hash map if this method returns
    /// `true`.
    fn is_fast() -> bool {
        false
    }
}

/// An empty `BlobIndex`.
#[derive(Debug, Clone, Copy)]
pub struct NoBlobIndex;

impl BlobIndex for NoBlobIndex {
    type ChunkForBlob = future::Ready<io::Result<Option<Uuid>>>;

    fn chunk_for_blob(self: &Pin<Arc<Self>>, _blob_id: Uuid) -> Self::ChunkForBlob {
        future::ready(Ok(None))
    }

    fn is_fast() -> bool {
        true
    }
}

/// An in-memory implementation of `BlobIndex`.
#[derive(Debug, Clone)]
pub struct MemBlobIndex(HashMap<Uuid, Uuid>);

impl MemBlobIndex {
    /// Construct a `MemBlobIndex` from an iterator of `(blob ID, chunk ID)`.
    pub fn new(blob_chunk: impl IntoIterator<Item = (Uuid, Uuid)>) -> Self {
        Self(blob_chunk.into_iter().collect())
    }
}

impl BlobIndex for MemBlobIndex {
    type ChunkForBlob = future::Ready<io::Result<Option<Uuid>>>;

    fn chunk_for_blob(self: &Pin<Arc<Self>>, blob_id: Uuid) -> Self::ChunkForBlob {
        future::ready(Ok(self.0.get(&blob_id).cloned()))
    }

    fn is_fast() -> bool {
        true
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
    type GetChunkStream: Future<Output = io::Result<Option<(Self::ChunkHdr, Self::ChunkRead)>>>
        + 'static;

    type ChunkHdr: ChunkHdr;

    /// A stream for reading chunk data.
    type ChunkRead: ReadSeek;

    /// Open a chunk for streamed read.
    fn get_chunk_stream(self: &Pin<Arc<Self>>, chunk_id: Uuid) -> Self::GetChunkStream;
}

/// A helper trait for simulating `dyn AsyncRead + Seek`.
pub trait ReadSeek: AsyncRead + Seek {}

// TODO: Use `AsyncSeek` when `futures` has one
//       Related PR: https://github.com/tokio-rs/tokio/pull/785

impl<T: ?Sized + AsyncRead + Seek> ReadSeek for T {}

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

impl<OC: OpenChunk> ChunkStore for StdChunkStore<OC>
where
    OC: 'static,
    OC::ReadSeek: 'static,
{
    type ChunkHdr = ChunkHdrReader<Box<[u8]>>;
    type ChunkRead = ReadWindow<OC::ReadSeek>;
    #[cfg(not(rustdoc))]
    existential type GetChunkStream: Future<
            Output = io::Result<Option<(ChunkHdrReader<Box<[u8]>>, ReadWindow<OC::ReadSeek>)>>,
        > + 'static;

    /// This associated type is actually defined using `existential type` but
    /// documented as a normal `type` as a work-around for this rustdoc-related
    /// bug: <https://github.com/rust-lang/rust/issues/58624>
    #[cfg(rustdoc)]
    type GetChunkStream = Box<dyn Future<
            Output = io::Result<Option<(ChunkHdrReader<Box<[u8]>>, ReadWindow<OC::ReadSeek>)>>,
        > + std::marker::Unpin + 'static>;

    fn get_chunk_stream(self: &Pin<Arc<Self>>, chunk_id: Uuid) -> Self::GetChunkStream {
        let stream = self.open_chunk.open_chunk(chunk_id);
        async move {
            let stream = if let Some(x) = stream? {
                x
            } else {
                return Ok(None);
            };

            let (data_reader, hdr) = await!(async_chunk_reader(stream))?;

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
}

// TODO: Add async version of `StdChunkStore`

/// The trait used by `StdChunkStore` to open a raw byte stream of the standard
/// chunk format.
pub trait OpenChunk {
    type ReadSeek: ReadSeek;
    fn open_chunk(&self, chunk_id: Uuid) -> io::Result<Option<Self::ReadSeek>>;
}
