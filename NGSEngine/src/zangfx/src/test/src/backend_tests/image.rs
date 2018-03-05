//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use gfx;
use common::BinaryInteger;
use super::{utils, TestDriver};

fn try_all_memory_types(device: &gfx::Device, builder: &mut gfx::ImageBuilder) {
    println!("  - Creating an image");
    let mut image = utils::UniqueImage::new(device, builder.build().unwrap());

    println!("  - Querying the memory requirement for the image");
    let req = device.get_memory_req((&*image).into()).unwrap();
    println!("  - Memory requirement = {:?}", req);

    for memory_type in req.memory_types.one_digits() {
        println!("  - Trying the memory type '{}'", memory_type);
        println!("    - Creating a heap");

        let heap = {
            let mut builder = device.build_dedicated_heap();
            builder.memory_type(memory_type);
            builder.prebind((&*image).into());
            builder.build().unwrap()
        };

        println!("    - Allocating a storage for the image");
        heap.bind((&*image).into())
            .expect("'bind' failed")
            .expect("allocation failed");

        println!("    - Creating an image view");
        {
            let image_view = device.build_image_view().image(&*image).build().unwrap();
            utils::UniqueImageView::new(device, image_view);
        }

        println!("  - Recreating a image");
        image = utils::UniqueImage::new(device, builder.build().unwrap());
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

            let mut usage = flags![gfx::ImageUsage::{}];

            if caps.contains(gfx::ImageFormatCaps::Render) {
                usage |= gfx::ImageUsage::Render;
            }
            if caps.contains(gfx::ImageFormatCaps::CopyRead) {
                usage |= gfx::ImageUsage::CopyRead;
            }
            if caps.contains(gfx::ImageFormatCaps::CopyWrite) {
                usage |= gfx::ImageUsage::CopyWrite;
            }
            if caps.contains(gfx::ImageFormatCaps::Sampled) {
                usage |= gfx::ImageUsage::Sampled;
            }
            if caps.contains(gfx::ImageFormatCaps::Storage) {
                usage |= gfx::ImageUsage::Storage;
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

        println!("- (1D + mip, {:?})", format);
        try_all_memory_types(
            device,
            device
                .build_image()
                .extents(&[32])
                .num_mip_levels(6)
                .format(format),
        );

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
