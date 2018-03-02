//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use gfx;
use super::TestDriver;

fn arg_table_sig_create<T: TestDriver>(driver: T, arg_type: gfx::ArgType) {
    driver.for_each_device(&mut |device| {
        let mut builder = device.build_arg_table_sig();
        builder.arg(0, arg_type).set_len(4);
        builder.build().unwrap();
    });
}

pub fn arg_table_sig_create_image<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::StorageImage)
}

pub fn arg_table_sig_create_buffer<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::StorageBuffer)
}

pub fn arg_table_sig_create_sampler<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::Sampler)
}

fn arg_table<T: TestDriver>(driver: T, arg_type: gfx::ArgType) {
    driver.for_each_device(&mut |device| {
        const TABLE_COUNT: usize = 4;

        let mut builder = device.build_arg_table_sig();
        builder.arg(0, arg_type).set_len(4);
        let sig = builder.build().unwrap();

        println!("- Allocating a pool with deallocation disabled");
        {
            let mut pool: Box<gfx::ArgPool> = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool.new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();
        }

        println!("- Allocating a pool with deallocation enabled");
        {
            let mut pool: Box<gfx::ArgPool> = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .enable_destroy_tables()
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool.new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool.new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();
        }
    });
}

pub fn arg_table_image<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::StorageImage)
}

pub fn arg_table_buffer<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::StorageBuffer)
}

pub fn arg_table_sampler<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::Sampler)
}
