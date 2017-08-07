//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsgfx as gfx;
extern crate cgmath;

pub use gfx::prelude::*;
pub use gfx::core;
pub use gfx::wsi;

pub use cgmath::Vector2;

pub use wsi::{SwapchainDescription, Window, Drawable, DrawableInfo, Swapchain};

use core::ImageFormat;
use wsi::{DefaultWindow, NewWindow, winit, ColorSpace, SwapchainError, FrameDescription};
use gfx::backends::{DefaultEnvironment, DefaultBackend};

pub trait App<B: Backend> {
    fn handle_event(&mut self, _: &winit::Event) -> bool {
        false
    }
    fn update_drawable_info(&mut self, _: &DrawableInfo) {}
    fn frame_description(&mut self) -> FrameDescription {
        FrameDescription {
            acquiring_engines: core::DeviceEngine::Universal.into(),
            releasing_engines: core::DeviceEngine::Universal.into(),
        }
    }
    fn render_to(&mut self, drawable: &Drawable<Backend = B>, drawable_info: &DrawableInfo);
    fn wait_completion(&mut self);
}

pub trait AppFactory: Sized {
    fn run<W: Window>(w: &W) -> Box<App<W::Backend>>;
    fn desired_formats() -> Vec<(Option<ImageFormat>, Option<ColorSpace>)> {
        vec![
            (
                Some(ImageFormat::SrgbBgra8),
                Some(ColorSpace::SrgbNonlinear)
            ),
            (
                Some(ImageFormat::SrgbRgba8),
                Some(ColorSpace::SrgbNonlinear)
            ),
            (None, Some(ColorSpace::SrgbNonlinear)),
        ]
    }
    fn image_usage() -> core::ImageUsageFlags {
        core::ImageUsage::ColorAttachment.into()
    }
}

struct Runner<W: Window> {
    window: W,
    app: Box<App<W::Backend>>,
}

impl<W: Window> Runner<W> {
    fn run(mut self, events_loop: &mut winit::EventsLoop) {
        let mut running = true;

        DefaultBackend::autorelease_pool_scope(|mut arp| while running {
            events_loop.poll_events(|event| {
                if self.app.handle_event(&event) {
                    return;
                }
                match event {
                    winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => {
                        running = false;
                    }
                    winit::Event::WindowEvent {
                        event: winit::WindowEvent::Resized(_, _), ..
                    } => {
                        // Very likely that the swapchain is now out-of-date.
                        // Update the swacphain.
                        self.update_view();
                    }
                    _ => (),
                }
            });

            self.update();
            arp.drain();
        });

        // We have to Wait for the completion because we have to ensure all uses of
        // swapchain images are completed before destroying the swapchain.
        DefaultBackend::autorelease_pool_scope(|_| { self.app.wait_completion(); });
    }

    fn update_view(&mut self) {
        // We have to Wait for the completion because we have to ensure all uses of
        // swapchain images are completed before updating the swapchain.
        self.app.wait_completion();
        self.window.update_swapchain();
        self.app.update_drawable_info(
            &self.window.swapchain().drawable_info(),
        );
    }

    fn update(&mut self) {
        loop {
            {
                let swapchain = self.window.swapchain();
                let frame = self.app.frame_description();
                let drawable = swapchain.next_drawable(&frame);

                match drawable {
                    Ok(drawable) => {
                        self.app.render_to(&drawable, &swapchain.drawable_info());
                        return;
                    }
                    Err(SwapchainError::OutOfDate) => {
                        // The swapchain is out of date. Need to update the swapchain
                        // to match the latest state.
                    }
                    Err(e) => {
                        panic!("Failed to acquire the next drawable.: {:?}", e);
                    }
                }
            }

            self.update_view();
        }
    }
}

pub fn run_example<A: AppFactory>() {
    use gfx::core::{Environment, InstanceBuilder};

    DefaultBackend::autorelease_pool_scope(|_| {
        let mut events_loop = winit::EventsLoop::new();
        let builder = winit::WindowBuilder::new();

        let mut instance_builder = <DefaultEnvironment as Environment>::InstanceBuilder::new()
            .expect("InstanceBuilder::new() have failed");
        DefaultWindow::modify_instance_builder(&mut instance_builder);
        instance_builder.enable_debug_report(
            core::DebugReportType::Information | core::DebugReportType::Warning |
                core::DebugReportType::PerformanceWarning |
                core::DebugReportType::Error,
            gfx::debug::report::TermStdoutDebugReportHandler::new(),
        );
        instance_builder.enable_validation();
        instance_builder.enable_debug_marker();

        let instance = instance_builder.build().expect(
            "InstanceBuilder::build() have failed",
        );

        let desired_formats = A::desired_formats();
        let sc_desc = SwapchainDescription {
            desired_formats: &desired_formats,
            image_usage: A::image_usage(),
        };
        let window = DefaultWindow::new(builder, &events_loop, &instance, &sc_desc).unwrap();

        let app = A::run(&window);

        let runner = Runner { window, app };
        runner.run(&mut events_loop);

        println!("Exiting...");
    });
}
