//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::TestDriver;

pub fn cmdqueue_create<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        use std::cmp::min;

        let queue_families = device.caps().queue_families();
        println!(
            "- {} queue families are defined by the device.",
            queue_families.len()
        );

        for (i, queue_family) in queue_families.iter().enumerate() {
            let count = min(queue_family.count, 16);
            println!(
                "- Creating {} queues for [{}] : {:?}",
                count, i, queue_family
            );
            for k in 0..count {
                println!("  - {} of {}", k + 1, count);
                device
                    .build_cmd_queue()
                    .queue_family(i as _)
                    .build()
                    .unwrap();
            }
        }
    });
}
