//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate test;
extern crate ngspf;

use self::test::Bencher;
use ngspf::context::{Context, PropertyProducerWrite};
use ngspf::viewport::LayerBuilder;

fn run_single(b: &mut Bencher, num_nodes: usize) {
    let context = Context::new();
    let layers: Vec<_> = (0..num_nodes)
        .map(|_| LayerBuilder::new().build(&context))
        .collect();
    b.iter(move || {
        // Production
        {
            let mut lock = context.lock_producer_frame().unwrap();

            for layer in layers.iter() {
                layer.opacity().set(&mut lock, 1.0).unwrap();
            }
        }
        context.commit().unwrap();

        // Presentation
        assert_eq!(context.num_pending_frames(), 1);
        context.lock_presenter_frame().unwrap();
    });
}

#[bench]
fn update_nodes_1000(b: &mut Bencher) {
    run_single(b, 1000)
}

#[bench]
fn update_nodes_100000(b: &mut Bencher) {
    run_single(b, 100000)
}
