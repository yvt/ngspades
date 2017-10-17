//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngspf;

use std::sync::Arc;

use ngspf::workspace::Workspace;
use ngspf::window::WindowBuilder;
use ngspf::prelude::*;

fn main() {
    let mut ws = Workspace::new().expect("failed to create a workspace");
    let context = Arc::clone(ws.context());

    // Produce the first frame
    {
        let mut frame = context.lock_producer_frame().expect(
            "failed to acquire a producer frame",
        );

        let window = WindowBuilder::new().build(&context);
        ws.root()
            .windows()
            .set(&mut frame, Some(window.into_node_ref()))
            .expect("failed to set the value of proeprty 'windows'");
    }
    context.commit().expect("failed to commit a frame");

    // Start the main loop
    ws.enter_main_loop().expect(
        "error occured while running the main loop",
    );
}
