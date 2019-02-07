//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::TestDriver;
use flags_macro::flags;
use zangfx_base as gfx;
use zangfx_common::BinaryInteger;

fn try_all_memory_types(device: &gfx::DeviceRef, builder: &mut dyn gfx::ImageBuilder) {
    println!("  - Creating an image");
    let mut image = builder.build().unwrap();

    println!("  - Querying the memory requirement for the image");
    let req = image.get_memory_req().unwrap();
    println!("  - Memory requirement = {:?}", req);

    for memory_type in req.memory_types.one_digits() {
        println!("  - Trying the memory type '{}'", memory_type);

        let heap = device.global_heap(memory_type);

        println!("    - Allocating a storage for the image");
        assert!(
            heap.bind((&image).into()).expect("'bind' failed"),
            "allocation failed"
        );

        println!("  - Recreating a image");
        image = builder.build().unwrap();
    }
}

pub fn image_all_formats<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        for &format in gfx::ImageFormat::values().iter() {
            let caps = device.caps().image_format_caps(format);
            println!("- (2D, {:?}): {:?}", format, caps);
            if caps.is_empty() {
                println!("  - Skipped -- no hardware/backend support");
                continue;
            }

            let mut usage = flags![gfx::ImageUsageFlags::{}];

            if caps.contains(gfx::ImageFormatCapsFlags::RENDER) {
                usage |= gfx::ImageUsageFlags::RENDER;
            }
            if caps.contains(gfx::ImageFormatCapsFlags::COPY_READ) {
                usage |= gfx::ImageUsageFlags::COPY_READ;
            }
            if caps.contains(gfx::ImageFormatCapsFlags::COPY_WRITE) {
                usage |= gfx::ImageUsageFlags::COPY_WRITE;
            }
            if caps.contains(gfx::ImageFormatCapsFlags::SAMPLED) {
                usage |= gfx::ImageUsageFlags::SAMPLED;
            }
            if caps.contains(gfx::ImageFormatCapsFlags::STORAGE) {
                usage |= gfx::ImageUsageFlags::STORAGE;
            }

            try_all_memory_types(
                device,
                device
                    .build_image()
                    .extents(&[32, 32])
                    .usage(usage)
                    .format(format),
            );
        }
    });
}

pub fn image_all_types<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let format = gfx::ImageFormat::SrgbRgba8;

        println!("- (1D, {:?})", format);
        try_all_memory_types(device, device.build_image().extents(&[32]).format(format));

        println!("- (2D, {:?})", format);
        try_all_memory_types(
            device,
            device.build_image().extents(&[32, 32]).format(format),
        );

        println!("- (2D + mip, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents(&[32, 32])
                .num_mip_levels(6)
                .format(format),
        );

        println!("- (2D array, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents(&[32, 32])
                .num_layers(Some(32))
                .format(format),
        );

        println!("- (2D array + mip, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents(&[32, 32])
                .num_layers(Some(32))
                .num_mip_levels(6)
                .format(format),
        );

        println!("- (3D, {:?})", format);
        try_all_memory_types(
            device,
            device.build_image().extents(&[32, 32, 32]).format(format),
        );

        println!("- (3D + mip, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents(&[32, 32, 32])
                .num_mip_levels(6)
                .format(format),
        );

        println!("- (Cube, {:?})", format);
        try_all_memory_types(device, device.build_image().extents_cube(32).format(format));

        println!("- (Cube + mip, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents_cube(32)
                .num_mip_levels(6)
                .format(format),
        );

        println!("- (Cube array, {:?})", format);
        if device.caps().limits().supports_cube_array {
            try_all_memory_types(
                device,
                device
                    .build_image()
                    .extents_cube(32)
                    .num_layers(Some(32))
                    .format(format),
            );
        } else {
            println!("  - Skipped -- no hardware/backend support");
        }

        println!("- (Cube array + mip, {:?})", format);
        if device.caps().limits().supports_cube_array {
            try_all_memory_types(
                device,
                device
                    .build_image()
                    .extents_cube(32)
                    .num_layers(Some(32))
                    .num_mip_levels(6)
                    .format(format),
            );
        } else {
            println!("  - Skipped -- no hardware/backend support");
        }
    });
}
