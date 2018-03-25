//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

use atomic_refcell::AtomicRefCell;
use iterpool::{intrusive_list, Pool, PoolPtr};

use gfx;
use gfx::prelude::*;

/// Temporary resource pool (duh).
///
/// # Resource States
///
///  - **Active**: The resource is currently in use (in the timeline of CB
///    encoding).
///     - Moves to **Free** when the resource was explicitly freed, or when
///       the current frame was finalized.
///  - **Free**: The resource is not currently in use but still resides in the
///    memory.
///     - Moves to **Active** if a resource request with the matching
///       parameters was made.
///     - Moves to **Finish** if it was not used in a frame at all.
///  - **Finish**: The resource will be freed soon.
///     - The resource will be destroyed upon the confirmation of the execution
///       completion of the last frame where it was used.
///
#[derive(Debug)]
pub struct TempResPool<B: Backend> {
    device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
    pool: Pool<ResInfo<B>>,
    unfinished_res_list: intrusive_list::ListHead,
    free_res_list_map: HashMap<ResDesc, intrusive_list::ListHead>,

    frames: VecDeque<Frame<B>>,
    start_frame_id: u64,
}

#[derive(Debug)]
struct Frame<B: Backend> {
    cb: Arc<AtomicRefCell<B::CommandBuffer>>,
    finished_res_list: intrusive_list::ListHead,
}

#[derive(Debug)]
struct ResInfo<B: Backend> {
    /// The last pipeline stage the resource was accessed by.
    stage: gfx::core::PipelineStageFlags,

    /// The last access type the resource was accessed with.
    access_type: gfx::core::AccessTypeFlags,

    data: ResData<B>,

    alloc: <B::UniversalHeap as MappableHeap>::Allocation,

    /// The last frame the resource was accessed in.
    last_use_frame_id: u64,

    /// `Link` for `unfinished_res_list` or `finished_res_list`
    link: Option<intrusive_list::Link>,

    /// `Link` for `free_res_list_map`
    free_res_link: Option<intrusive_list::Link>,
}

#[derive(Debug)]
enum ResData<B: Backend> {
    Image(
        B::Image,
        gfx::core::ImageLayout,
        gfx::core::ImageDescription,
    ),
    Buffer(B::Buffer, gfx::core::BufferDescription),
}

impl<B: Backend> ResData<B> {
    fn desc(&self) -> ResDesc {
        match self {
            &ResData::Image(_, _, ref desc) => ResDesc::Image(desc.clone()),
            &ResData::Buffer(_, ref desc) => ResDesc::Buffer(desc.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ResDesc {
    Image(gfx::core::ImageDescription),
    Buffer(gfx::core::BufferDescription),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TempImage<B: Backend>(PoolPtr, B::Image);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TempBuffer<B: Backend>(PoolPtr, B::Buffer);

impl<B: Backend> TempImage<B> {
    pub fn image(&self) -> &B::Image {
        &self.1
    }
}

impl<B: Backend> TempBuffer<B> {
    pub fn buffer(&self) -> &B::Buffer {
        &self.1
    }

    pub fn into_buffer(self) -> B::Buffer {
        self.1
    }
}

pub struct TempResOp<'a, B: Backend>(&'a mut TempResPool<B>, PoolPtr);

impl<B: Backend> TempResPool<B> {
    pub fn new(
        device: Arc<B::Device>,
        heap: Arc<Mutex<B::UniversalHeap>>,
    ) -> gfx::core::Result<Self> {
        Ok(Self {
            device,
            heap,

            pool: Pool::new(),
            unfinished_res_list: intrusive_list::ListHead::new(),
            free_res_list_map: HashMap::new(),
            frames: VecDeque::new(),
            start_frame_id: 1,
        })
    }

    pub fn allocate_image(
        &mut self,
        desc: &gfx::core::ImageDescription,
    ) -> gfx::core::Result<TempImage<B>> {
        let p = self.allocate(ResDesc::Image(desc.clone()))?;
        match self.pool[p].data {
            ResData::Image(ref image, _, _) => Ok(TempImage(p, image.clone())),
            _ => unreachable!(),
        }
    }

    pub fn allocate_buffer(
        &mut self,
        desc: &gfx::core::BufferDescription,
    ) -> gfx::core::Result<TempBuffer<B>> {
        let p = self.allocate(ResDesc::Buffer(desc.clone()))?;
        match self.pool[p].data {
            ResData::Buffer(ref buffer, _) => Ok(TempBuffer(p, buffer.clone())),
            _ => unreachable!(),
        }
    }

    pub fn ops_image(&mut self, image: &TempImage<B>) -> TempResOp<B> {
        TempResOp(self, image.0)
    }

    pub fn ops_buffer(&mut self, buffer: &TempBuffer<B>) -> TempResOp<B> {
        TempResOp(self, buffer.0)
    }

    fn allocate(&mut self, desc: ResDesc) -> gfx::core::Result<PoolPtr> {
        let current_frame_id = self.start_frame_id + self.frames.len() as u64;

        // Try `free_res_list_map` first
        if let Some(free_res_list) = self.free_res_list_map.get_mut(&desc) {
            let p = free_res_list
                .accessor_mut(&mut self.pool, |r| &mut r.free_res_link)
                .pop_front()
                .unwrap();
            self.pool[p].last_use_frame_id = current_frame_id;
            return Ok(p);
        }

        // Must do a real allocation
        let (alloc, data) = {
            let mut heap = self.heap.lock().unwrap();
            match desc {
                ResDesc::Image(desc) => {
                    let (alloc, image) = heap.make_image(&desc)?.unwrap();
                    (
                        alloc,
                        ResData::Image(image, gfx::core::ImageLayout::Undefined, desc),
                    )
                }
                ResDesc::Buffer(desc) => {
                    let (alloc, buffer) = heap.make_buffer(&desc)?.unwrap();
                    (alloc, ResData::Buffer(buffer, desc))
                }
            }
        };

        let p = self.pool.allocate(ResInfo {
            stage: gfx::core::PipelineStage::TopOfPipe.into(),
            access_type: gfx::core::AccessType::MemoryWrite.into(),
            data,
            alloc,
            last_use_frame_id: current_frame_id,
            link: None,
            free_res_link: None,
        });

        self.unfinished_res_list
            .accessor_mut(&mut self.pool, |r| &mut r.link)
            .push_back(p);

        Ok(p)
    }

    fn deallocate(&mut self, p: PoolPtr) {
        debug_assert_eq!(
            self.pool[p].last_use_frame_id,
            self.start_frame_id + self.frames.len() as u64,
            "The resource was not used in the current frame"
        );

        let desc = self.pool[p].data.desc();
        let ref mut free_res_list = self.free_res_list_map
            .entry(desc)
            .or_insert_with(Default::default);
        free_res_list
            .accessor_mut(&mut self.pool, |r| &mut r.free_res_link)
            .push_back(p);
    }

    /// Encode resource barriers required to release the allocated memory region
    /// correctly. Also mark the end of the current frame which is used as a
    /// granularity of the lifetime tracking.
    pub fn finalize_frame(
        &mut self,
        cb_cell: Arc<AtomicRefCell<B::CommandBuffer>>,
        cb: &mut B::CommandBuffer,
    ) {
        {
            let mut ptr = self.unfinished_res_list.first;
            while let Some(p) = ptr {
                ptr = Some(self.pool[p].link.unwrap().next);
                if ptr == self.unfinished_res_list.first {
                    ptr = None;
                }

                let last_use_frame_index =
                    (self.pool[p].last_use_frame_id - self.start_frame_id) as usize;

                if last_use_frame_index == self.frames.len() {
                    // The resource was used in the current frame
                    if self.pool[p].free_res_link.is_none() {
                        // Force free
                        let desc = self.pool[p].data.desc();
                        let ref mut free_res_list = self.free_res_list_map
                            .entry(desc)
                            .or_insert_with(Default::default);
                        free_res_list
                            .accessor_mut(&mut self.pool, |r| &mut r.free_res_link)
                            .push_back(p);
                    }

                    let ref res_info = self.pool[p];
                    cb.resource_barrier(
                        res_info.stage,
                        res_info.access_type,
                        gfx::core::PipelineStage::BottomOfPipe.into(),
                        gfx::core::AccessType::MemoryWrite.into(),
                        &match res_info.data {
                            ResData::Image(ref image, layout, _) => {
                                gfx::core::SubresourceWithLayout::Image {
                                    image,
                                    range: Default::default(),
                                    old_layout: layout,
                                    new_layout: layout,
                                }
                            }
                            ResData::Buffer(ref buffer, ref desc) => {
                                gfx::core::SubresourceWithLayout::Buffer {
                                    buffer,
                                    offset: 0,
                                    len: desc.size,
                                }
                            }
                        },
                    );
                } else {
                    // Kill this resource
                    self.unfinished_res_list
                        .accessor_mut(&mut self.pool, |r| &mut r.link)
                        .remove(p);

                    let ref mut frame = self.frames[last_use_frame_index];
                    frame
                        .finished_res_list
                        .accessor_mut(&mut self.pool, |r| &mut r.link)
                        .push_back(p);

                    if self.pool[p].free_res_link.is_some() {
                        let desc = self.pool[p].data.desc();
                        let rem = {
                            let mut free_res_list = self.free_res_list_map.get_mut(&desc).unwrap();
                            free_res_list
                                .accessor_mut(&mut self.pool, |r| &mut r.free_res_link)
                                .remove(p);
                            free_res_list.is_empty()
                        };
                        if rem {
                            self.free_res_list_map.remove(&desc);
                        }
                    }
                }
            }
        }

        // Push frame
        self.frames.push_back(Frame {
            cb: cb_cell,
            finished_res_list: intrusive_list::ListHead::new(),
        });
    }

    pub fn retire_old_frames(&mut self) {
        let mut heap = None;

        // Retire frames
        while self.frames.len() > 1 {
            use gfx::core::CommandBufferState::*;
            let state = self.frames[0].cb.borrow().state();
            match state {
                Initial | Recording | Executable => {
                    unreachable!();
                }
                Pending => {
                    break;
                }
                Completed | Error => {
                    let mut frame = self.frames.pop_front().unwrap();
                    self.start_frame_id += 1;

                    while let Some(p) = frame
                        .finished_res_list
                        .accessor_mut(&mut self.pool, |r| &mut r.link)
                        .pop_front()
                    {
                        let res: ResInfo<B> = self.pool.deallocate(p).unwrap();

                        if heap.is_none() {
                            heap = Some(self.heap.lock().unwrap());
                        }
                        heap.as_mut().unwrap().deallocate(res.alloc);
                    }
                }
            }
        }
    }
}

impl<'a, B: Backend> TempResOp<'a, B> {
    /// Mark that this temporary resource is no longer used in this frame.
    ///
    /// Note that deallocation is optional. All temporary resources are marked
    /// as free on the end of each frame.
    pub fn deallocate(&mut self) {
        self.0.deallocate(self.1);
    }

    pub fn allocation_mut(&mut self) -> &mut <B::UniversalHeap as MappableHeap>::Allocation {
        &mut self.0.pool[self.1].alloc
    }

    pub fn stage_access_type_mut(
        &mut self,
    ) -> (
        &mut gfx::core::PipelineStageFlags,
        &mut gfx::core::AccessTypeFlags,
    ) {
        let ref mut res_info: ResInfo<B> = self.0.pool[self.1];
        (&mut res_info.stage, &mut res_info.access_type)
    }

    pub fn image_layout_mut(&mut self) -> Option<&mut gfx::core::ImageLayout> {
        let ref mut res_info: ResInfo<B> = self.0.pool[self.1];
        match res_info.data {
            ResData::Image(_, ref mut layout, _) => Some(layout),
            _ => None,
        }
    }
}
