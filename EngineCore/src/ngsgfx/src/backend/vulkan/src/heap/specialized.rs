//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;
use std::sync::Arc;

use {RefEqArc, RefEqBox, DeviceRef, Backend, translate_map_memory_error_unwrap, AshDevice,
     translate_generic_error_unwrap};
use imp::{Buffer, Image, DeviceData, UnassociatedImage, UnassociatedBuffer};
use super::hunk::MemoryHunk;
use super::suballoc::{Suballocator, SuballocatorRegion};

pub struct SpecializedHeap<T: DeviceRef> {
    data: RefEqBox<SpecializedHeapData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for SpecializedHeap<T> => data
}

#[derive(Debug)]
struct SpecializedHeapData<T: DeviceRef> {
    device_data: Arc<DeviceData<T>>,
    heap_id: RefEqArc<()>,
    usage: core::SpecializedHeapUsage,
    hunk: Arc<MemoryHunk<T>>,
    sa: Suballocator,
    host_visible: bool,
    memory_type: u8,
}

#[derive(Debug)]
pub struct SpecializedHeapAllocation {
    id: (RefEqArc<()>, SuballocatorRegion),
    host_visible: bool,
}

derive_using_field! {
    (); (PartialEq) for SpecializedHeapAllocation => id
}

impl core::Allocation for SpecializedHeapAllocation {
    fn is_mappable(&self) -> bool {
        self.host_visible
    }
}

impl<T: DeviceRef> core::Heap<Backend<T>> for SpecializedHeap<T> {
    fn make_buffer(
        &mut self,
        description: &core::BufferDescription,
    ) -> core::Result<Option<(Self::Allocation, Buffer<T>)>> {
        use ngsgfx_common::int::BinaryInteger;
        let ref mut data = *self.data;

        debug_assert!(data.usage.supports_buffer(description));

        let proto = UnassociatedBuffer::new(&data.device_data.device_ref, description)?;
        let req = proto.memory_requirements();

        assert!(req.memory_type_bits.get_bit(data.memory_type as u32));

        let region = data.sa.allocate(req.size, req.alignment);
        if let Some(region) = region {
            match proto.associate(data.hunk.clone(), region.offset()) {
                Ok(obj) => Ok(Some((
                    SpecializedHeapAllocation {
                        id: (data.heap_id.clone(), region),
                        host_visible: data.host_visible,
                    },
                    obj,
                ))),
                Err(err) => {
                    data.sa.deallocate(region);
                    Err(err)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn make_image(
        &mut self,
        description: &core::ImageDescription,
    ) -> core::Result<Option<(Self::Allocation, Image<T>)>> {
        use ngsgfx_common::int::BinaryInteger;
        let ref mut data = *self.data;

        debug_assert!(data.usage.supports_image(description));

        let proto = UnassociatedImage::new(&data.device_data.device_ref, description)?;
        let req = proto.memory_requirements();

        assert!(req.memory_type_bits.get_bit(data.memory_type as u32));

        let region = data.sa.allocate(req.size, req.alignment);
        if let Some(region) = region {
            match proto.associate(data.hunk.clone(), region.offset()) {
                Ok(obj) => Ok(Some((
                    SpecializedHeapAllocation {
                        id: (data.heap_id.clone(), region),
                        host_visible: data.host_visible,
                    },
                    obj,
                ))),
                Err(err) => {
                    data.sa.deallocate(region);
                    Err(err)
                }
            }
        } else {
            Ok(None)
        }
    }
}

impl<T: DeviceRef> SpecializedHeap<T> {
    pub(crate) fn new(
        device_data: Arc<DeviceData<T>>,
        description: &core::SpecializedHeapDescription,
    ) -> core::Result<Self> {
        let (req, storage_mode) = match description.usage {
            core::SpecializedHeapUsage::Buffers {
                storage_mode,
                usage,
            } => {
                let proto = UnassociatedBuffer::new(
                    &device_data.device_ref,
                    &core::BufferDescription {
                        storage_mode,
                        usage,
                        size: 1,
                    },
                )?;
                (proto.memory_requirements(), storage_mode)
            }
            core::SpecializedHeapUsage::Images {
                storage_mode,
                flags,
                usage,
                format,
                tiling,
            } => {
                let proto = UnassociatedImage::new(
                    &device_data.device_ref,
                    &core::ImageDescription {
                        storage_mode,
                        flags,
                        usage,
                        format,
                        tiling,
                        image_type: core::ImageType::TwoD,
                        ..core::ImageDescription::default()
                    },
                )?;
                (proto.memory_requirements(), storage_mode)
            }
        };

        let mem_type = device_data
            .cfg
            .storage_mode_mappings
            .map_storage_mode(storage_mode, req.memory_type_bits)
            .expect("no suitable memory type");

        let host_visible = {
            let ref mem_type_info = device_data.cfg.memory_types[mem_type as usize].0;
            mem_type_info.property_flags.subset(
                vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT,
            )
        };

        let handle = unsafe {
            let ref device_ref = device_data.device_ref;
            device_ref.device().allocate_memory(
                &vk::MemoryAllocateInfo {
                    s_type: vk::StructureType::MemoryAllocateInfo,
                    p_next: ptr::null(),
                    allocation_size: description.size,
                    memory_type_index: mem_type as u32,
                },
                device_ref.allocation_callbacks(),
            )
        }.map_err(translate_generic_error_unwrap)?;

        let hunk = unsafe { MemoryHunk::from_raw(&device_data.device_ref, handle) };

        Ok(Self {
            data: RefEqBox::new(SpecializedHeapData {
                usage: description.usage,
                hunk: Arc::new(hunk),
                sa: Suballocator::new(description.size),
                host_visible,
                memory_type: mem_type,
                heap_id: RefEqArc::new(()),
                device_data,
            }),
        })
    }
}

impl<T: DeviceRef> core::MappableHeap for SpecializedHeap<T> {
    type Allocation = SpecializedHeapAllocation;
    type MappingInfo = ();
    fn make_aliasable(&mut self, allocation: &mut Self::Allocation) {
        assert_eq!(allocation.id.0, self.data.heap_id);

        self.data.sa.make_aliasable(&mut allocation.id.1);
    }
    fn deallocate(&mut self, allocation: Self::Allocation) {
        assert_eq!(allocation.id.0, self.data.heap_id);

        self.data.sa.deallocate(allocation.id.1);
    }
    unsafe fn raw_unmap_memory(&mut self, _: Self::MappingInfo) {
        let device: &AshDevice = self.data.device_data.device_ref.device();
        device.unmap_memory(self.data.hunk.handle());
    }
    unsafe fn raw_map_memory(
        &mut self,
        allocation: &mut Self::Allocation,
    ) -> core::Result<(*mut u8, usize, Self::MappingInfo)> {
        assert_eq!(allocation.id.0, self.data.heap_id);
        assert!(self.data.host_visible);

        let device: &AshDevice = self.data.device_data.device_ref.device();
        let ref region: SuballocatorRegion = allocation.id.1;

        let ptr = device
            .map_memory(
                self.data.hunk.handle(),
                region.offset(),
                region.size(),
                vk::MemoryMapFlags::empty(),
            )
            .map_err(translate_map_memory_error_unwrap)?;

        Ok((ptr as *mut u8, region.size() as usize, ()))
    }
}

impl<T: DeviceRef> core::Marker for SpecializedHeap<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}
