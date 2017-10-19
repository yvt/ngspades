//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet, VecDeque};

use cgmath::Vector3;
use atomic_refcell::AtomicRefCell;
use iterpool::{Pool, PoolPtr, intrusive_list};

use gfx;
use gfx::prelude::*;

use super::{ImageRef, ImageData, WorkspaceDevice, Layer, LayerContents};
use context::{NodeRef, PresenterFrame, for_each_node};
use prelude::*;

/// Manages residency of `ImageRef` on a NgsGFX device.
///
///  - First, uses of `ImageRef` are detected by the method `scan_nodes` which
///    inspects the `contents` of every `Layer`. Those images are added to the
///    internal set `image_uses`.
///  - TODO: persistent image group (manage ref-count collectively to maintain
///    the residency of a large number of images efficiently)
///  - If new images were found (i.e. not in the resident image set but
///    in `image_uses`), they are uploaded to the device by the method `upload`.
///    Uploading is done in a unit named *session*, which consists of one or
///    more images. Each session is associated with a staging buffer in a
///    host-visible memory region.
///  - `upload` also releases unused images.
///
/// ## Engine Mappings
///
///  - The `Universal` engine is used for layer contents because they are going
///    to be consumed by the `Universal` engine within the same frame.
///  - The `Copy` engine is used for preloading persistent image groups so they
///    can be uploaded asynchronously. They are transferred to the `Universal`
///    engine upon first actual uses.
///
/// Images loaded by `Uploader` only can consumed by the `Universal` engine.
#[derive(Debug)]
pub struct Uploader<B: Backend> {
    device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
    params: UploaderParams,
    images: Pool<ResidentImageData<B>>,
    image_map: HashMap<ImageRef, PoolPtr>,

    unused_image_list: intrusive_list::ListHead,

    image_uses: HashSet<ImageRef>,
    new_images_bytes: usize,
    new_images_list: Vec<ImageRef>,

    /// Temporarily used by `upload`.
    session_image_list: Vec<ImageRef>,

    /// The session ID offset of `sessions`.
    session_start_id: usize,

    /// The number of bytes being transfered. (Bounded by `max_bytes_ongoing`)
    /// Equals to `sessions.iter().map(|s| s.bytes).sum()`.
    ongoing_bytes: usize,

    /// Ongoing session FIFO.
    sessions: VecDeque<UploadSession<B>>,
}

#[derive(Debug)]
pub struct UploaderParams {
    /// The maximum number of bytes transferred per session.
    pub max_bytes_per_session: usize,

    /// The maximum number of total bytes of ongoing upload sessions.
    pub max_bytes_ongoing: usize,
}

impl Default for UploaderParams {
    fn default() -> Self {
        Self {
            max_bytes_per_session: 16_000_000,
            max_bytes_ongoing: 64_000_000,
        }
    }
}

#[derive(Debug)]
pub struct ResidentImage<'a, B: Backend> {
    uploader: &'a Uploader<B>,
    data: &'a ResidentImageData<B>,
}

#[derive(Debug)]
struct ResidentImageData<B: Backend> {
    image_ref: ImageRef,
    image: B::Image,
    image_view: B::ImageView,
    allocation: <B::UniversalHeap as MappableHeap>::Allocation,
    session_id: usize,
    ref_count: usize,

    // Linked to `unused_image_list` iff `ref_count == 0`
    unused_image_link: Option<intrusive_list::Link>,

    /// Indicates whether the `Universal` engine has the ownership of this
    /// resource.
    ownership_transfer_required: bool,
}

/// Represents a single upload operation of collective images.
#[derive(Debug)]
struct UploadSession<B: Backend> {
    command_buffer: Arc<AtomicRefCell<B::CommandBuffer>>,
    fence: B::Fence,
    allocation: <B::UniversalHeap as MappableHeap>::Allocation,
    bytes: usize,
}

fn bytes_of_image(image_ref: &ImageRef, frame: &PresenterFrame) -> usize {
    image_ref
        .image_data()
        .get_presenter_ref(frame)
        .unwrap()
        .pixels_u32()
        .len() * 4
}

impl<B: Backend> Uploader<B> {
    pub fn new(ws_device: &WorkspaceDevice<B>) -> gfx::core::Result<Self> {
        Ok(Self {
            heap: Arc::clone(ws_device.objects().heap()),
            device: Arc::clone(ws_device.objects().gfx_device()),
            params: UploaderParams::default(),
            images: Pool::new(),
            image_map: HashMap::new(),

            unused_image_list: intrusive_list::ListHead::new(),

            image_uses: HashSet::new(),
            new_images_bytes: 0,
            new_images_list: Vec::new(),

            session_image_list: Vec::new(),

            session_start_id: 0,
            ongoing_bytes: 0,
            sessions: VecDeque::new(),
        })
    }

    /// Clear `image_uses` and update `ResidentImageData::ref_count` and
    /// `unused_image_list` accordingly.
    pub fn clear_image_uses(&mut self) {
        for image_ref in self.image_uses.drain() {
            if let Some(&image_ptr) = self.image_map.get(&image_ref) {
                let new_ref_count = {
                    let ref mut image = self.images[image_ptr];
                    image.ref_count -= 1;
                    image.ref_count
                };
                if new_ref_count == 0 {
                    self.unused_image_list
                        .accessor_mut(&mut self.images, |i| &mut i.unused_image_link)
                        .push_back(image_ptr);
                }
            }
        }
    }

    /// Scan image uses in the specified node and:
    ///
    ///  - Add them to `image_uses`.
    ///  - If they are not in `images`, update `new_image_bytes` and also add
    ///    them to `new_images_list`.
    ///  - If they are, update `ResidentImageData::ref_count` as well as unlinking
    ///    them from `unused_image_list`.
    ///
    pub fn scan_nodes(&mut self, root: &NodeRef, frame: &PresenterFrame) {
        for_each_node(root, |node| {
            if let Some(layer) = node.downcast_ref::<Layer>() {
                // Scan recursively
                if let &Some(ref child) = layer.child.read_presenter(frame).unwrap() {
                    self.scan_nodes(child, frame);
                }

                // Check the layer contents
                match layer.contents.read_presenter(frame).unwrap() {
                    &LayerContents::Image(ref image_ref) => {
                        if self.image_uses.contains(image_ref) {
                            return;
                        }
                        self.image_uses.insert(ImageRef::clone(image_ref));

                        if let Some(&image_ptr) = self.image_map.get(image_ref) {
                            // The image is resident
                            let new_ref_count = {
                                let ref mut image = self.images[image_ptr];
                                image.ref_count += 1;
                                image.ref_count
                            };
                            if new_ref_count == 1 {
                                self.unused_image_list
                                    .accessor_mut(&mut self.images, |i| &mut i.unused_image_link)
                                    .remove(image_ptr);
                            }
                        } else {
                            // Newly uploaded image
                            self.new_images_bytes += bytes_of_image(image_ref, frame);
                            self.new_images_list.push(ImageRef::clone(image_ref));
                        }
                    }
                    _ => {}
                }
            } else {
                // Ignore an unknown node type
            }
        });
    }

    /// Return a command buffer for uploading images.
    ///
    /// This may submit command buffers by itself in some situations to
    /// fulfill the memory limit requirement.
    ///
    /// The returned command buffer **must** be submitted before the next call
    /// to `upload`.
    pub fn upload(
        &mut self,
        frame: &PresenterFrame,
    ) -> gfx::core::Result<Vec<Arc<AtomicRefCell<B::CommandBuffer>>>> {
        {
            let mut heap = None;

            // Unload unused images first
            // TODO: should wait until the device uses of images are done
            while let Some(i) = self.unused_image_list
                .accessor_mut(&mut self.images, |i| &mut i.unused_image_link)
                .pop_front()
            {
                if heap.is_none() {
                    heap = Some(self.heap.lock().unwrap());
                }
                let r_im = self.images.deallocate(i);
                heap.as_mut().unwrap().deallocate(r_im.unwrap().allocation);
            }

            // Check retirement of the ongoing sessions
            while self.sessions.len() > 0 {
                use gfx::core::CommandBufferState::*;
                let state = self.sessions[0].command_buffer.borrow().state();
                match state {
                    Initial | Recording | Executable => {
                        unreachable!();
                    }
                    Pending => {
                        break;
                    }
                    Completed | Error => {
                        let session = self.sessions.pop_front().unwrap();
                        self.ongoing_bytes -= session.bytes;

                        if heap.is_none() {
                            heap = Some(self.heap.lock().unwrap());
                        }
                        heap.as_mut().unwrap().deallocate(session.allocation);
                    }
                }
            }
        }

        if self.new_images_list.len() == 0 {
            return Ok(Vec::new());
        }

        let mut num_sessions_started = 0;

        while self.new_images_list.len() > 0 {
            // Create a list of images for the next session
            let mut size = 0;
            while self.new_images_list.len() > 0 {
                let image_size =
                    bytes_of_image(&self.new_images_list[self.new_images_list.len() - 1], frame);
                if size + image_size > self.params.max_bytes_per_session && size > 0 {
                    break;
                }
                size += image_size;
                self.session_image_list.push(
                    self.new_images_list.pop().unwrap(),
                );
            }

            // Retire ongoing sessions as needed to meet the `max_bytes_ongoing`
            // requirements
            self.reserve_session(size)?;

            // Start the session
            self.start_session(size, frame)?;
            num_sessions_started += 1;

            assert!(self.session_image_list.is_empty());
        }

        // Some new session may have been already submitted to the device and
        // removed from `self.sessions` by `reserve_session`
        if num_sessions_started > self.sessions.len() {
            num_sessions_started = self.sessions.len();
        }

        Ok(
            (0..num_sessions_started)
                .map(|i| {
                    Arc::clone(&self.sessions[self.sessions.len() - 1 - i].command_buffer)
                })
                .collect(),
        )
    }

    /// Start a new session that uploads images from `session_image_list`.
    fn start_session(&mut self, size: usize, frame: &PresenterFrame) -> gfx::core::Result<()> {
        assert!(!self.session_image_list.is_empty());

        // Construct a staging buffer
        let (allocation, buffer) = {
            let mut heap = self.heap.lock().unwrap();
            let (mut allocation, buffer) = heap.make_buffer(&gfx::core::BufferDescription {
                usage: gfx::core::BufferUsage::TransferSource.into(),
                size: size as gfx::core::DeviceSize,
                storage_mode: gfx::core::StorageMode::Shared,
            })?
                .unwrap();

            {
                use std::ptr::copy;
                let mut map = heap.map_memory(&mut allocation)?;
                let mut offs = 0;
                for image in self.session_image_list.iter() {
                    let image_data = image.image_data();
                    let image_data: &ImageData = image_data.get_presenter_ref(frame).unwrap();
                    let pixels = image_data.pixels_u32();
                    let size = pixels.len() * 4;
                    unsafe {
                        copy(
                            pixels.as_ptr() as *const u8,
                            map[offs..offs + size].as_mut_ptr(),
                            size,
                        );
                    }
                    offs += size;
                }
            }

            (allocation, buffer)
        };

        // Construct the command buffer (as well as image objects)
        let mut cb = self.device.main_queue().make_command_buffer()?;
        let fence = self.device.main_queue().make_fence(
            &gfx::core::FenceDescription {
                update_engines: gfx::core::DeviceEngine::Universal.into(),
                wait_engines: gfx::core::DeviceEngine::Universal.into(),
            },
        )?;
        let session_id = self.sessions.len() + self.session_start_id;

        cb.begin_encoding();
        cb.begin_copy_pass(gfx::core::DeviceEngine::Universal);
        cb.acquire_resource(
            gfx::core::PipelineStage::Transfer.into(),
            gfx::core::AccessType::TransferRead.into(),
            gfx::core::DeviceEngine::Host,
            &gfx::core::SubresourceWithLayout::Buffer {
                buffer: &buffer,
                offset: 0,
                len: size as gfx::core::DeviceSize,
            },
        );
        let mut offs = 0;
        for image_ref in self.session_image_list.drain(..) {
            let (size, desc) = {
                let image_data = image_ref.image_data();
                let image_data: &ImageData = image_data.get_presenter_ref(frame).unwrap();
                (
                    image_data.pixels_u32().len() * 4,
                    gfx::core::ImageDescription {
                        usage: gfx::core::ImageUsage::Sampled |
                            gfx::core::ImageUsage::TransferDestination,
                        format: gfx::core::ImageFormat::SrgbRgba8,
                        extent: image_data.size().cast().extend(1),
                        ..Default::default()
                    },
                )
            };
            // TODO: creation of image objects should be moved out; this is ridiculous
            let (allocation, image) = self.heap.lock().unwrap().make_image(&desc)?.unwrap();
            cb.resource_barrier(
                gfx::core::PipelineStageFlags::empty(),
                gfx::core::AccessTypeFlags::empty(),
                gfx::core::PipelineStage::FragmentShader.into(),
                gfx::core::AccessType::ShaderRead.into(),
                &gfx::core::SubresourceWithLayout::Image {
                    image: &image,
                    range: Default::default(),
                    old_layout: gfx::core::ImageLayout::Undefined,
                    new_layout: gfx::core::ImageLayout::TransferDestination,
                },
            );
            cb.copy_buffer_to_image(
                &buffer,
                &gfx::core::BufferImageRange {
                    offset: offs as gfx::core::DeviceSize,
                    row_stride: desc.extent.x as gfx::core::DeviceSize,
                    plane_stride: 0,
                },
                &image,
                gfx::core::ImageLayout::TransferDestination,
                gfx::core::ImageAspect::Color,
                &gfx::core::ImageSubresourceLayers {
                    mip_level: 0,
                    base_array_layer: 0,
                    num_array_layers: 1,
                },
                Vector3::new(0, 0, 0),
                desc.extent,
            );
            cb.end_debug_group();
            cb.resource_barrier(
                gfx::core::PipelineStage::Transfer.into(),
                gfx::core::AccessType::TransferWrite.into(),
                gfx::core::PipelineStage::FragmentShader.into(),
                gfx::core::AccessType::ShaderRead.into(),
                &gfx::core::SubresourceWithLayout::Image {
                    image: &image,
                    range: Default::default(),
                    old_layout: gfx::core::ImageLayout::TransferDestination,
                    new_layout: gfx::core::ImageLayout::ShaderRead,
                },
            );

            // Register to the resident image table
            let image_ptr = self.images.allocate(ResidentImageData {
                image_ref: ImageRef::clone(&image_ref),
                image_view: self.device.factory().make_image_view(
                    &gfx::core::ImageViewDescription {
                        image: &image,
                        image_type: desc.image_type,
                        format: desc.format,
                        range: Default::default(),
                    },
                )?,
                image,
                allocation,
                session_id,
                ref_count: 1,
                unused_image_link: None,
                ownership_transfer_required: false,
            });
            self.image_map.insert(image_ref, image_ptr);

            offs += size;
        }
        cb.update_fence(
            gfx::core::PipelineStage::Transfer.into(),
            gfx::core::AccessType::TransferWrite.into(),
            &fence,
        );
        cb.end_pass();
        cb.end_encoding()?;

        self.sessions.push_back(UploadSession {
            command_buffer: Arc::new(AtomicRefCell::new(cb)),
            allocation,
            fence,
            bytes: size,
        });
        self.ongoing_bytes += size;
        Ok(())
    }

    /// Retire ongoing sessions forcefully so new sessions can be started.
    fn reserve_session(&mut self, additional_ongoing_bytes: usize) -> gfx::core::Result<()> {
        while self.ongoing_bytes + additional_ongoing_bytes > self.params.max_bytes_ongoing &&
            self.sessions.len() > 0
        {
            self.force_retire_earliest_session()?;
        }
        Ok(())
    }

    fn force_retire_earliest_session(&mut self) -> gfx::core::Result<()> {
        let session = self.sessions.pop_front().unwrap();
        self.session_start_id += 1;
        self.ongoing_bytes -= session.bytes;

        {
            let cb = session.command_buffer.borrow();
            use gfx::core::CommandBufferState::*;
            use std::mem::drop;
            match cb.state() {
                Initial | Recording | Executable => {
                    // The CB has not been even returned by `upload` yet
                    drop(cb);
                    let mut cb = session.command_buffer.borrow_mut();
                    self.device.main_queue().submit_commands(
                        &mut [&mut *cb],
                        None,
                    )?;
                    cb.wait_completion()?;
                }
                _ => {
                    cb.wait_completion()?;
                }
            }
        }

        // Deallocate the staging buffer
        self.heap.lock().unwrap().deallocate(session.allocation);

        Ok(())
    }

    pub fn get(&self, image_ref: &ImageRef) -> Option<ResidentImage<B>> {
        self.image_map.get(image_ref).map(|&i| {
            ResidentImage {
                uploader: self,
                data: &self.images[i],
            }
        })
    }
}

impl<'a, B: Backend> ResidentImage<'a, B> {
    pub fn image(&self) -> &B::Image {
        &self.data.image
    }

    pub fn image_view(&self) -> &B::ImageView {
        &self.data.image_view
    }

    pub fn fence(&self) -> Option<&B::Fence> {
        if self.data.session_id >= self.uploader.session_start_id {
            Some(
                &self.uploader.sessions[self.data.session_id - self.uploader.session_start_id].fence,
            )
        } else {
            None
        }
    }
}
