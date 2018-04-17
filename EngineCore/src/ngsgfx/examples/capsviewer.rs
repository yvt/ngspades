//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

extern crate ngsgfx as gfx;
use gfx::core;
use gfx::prelude::*;

fn try_environment<K: core::Environment>() {
    use core::{InstanceBuilder, Instance, DeviceBuilder, Backend};
    <K::Backend as Backend>::autorelease_pool_scope(|_| {
        use core::Adapter;

        let mut inst_builder: K::InstanceBuilder = match K::InstanceBuilder::new() {
            Ok(i) => i,
            Err(e) => {
                println!("    InstanceBuilder::new() failed: {:?}", e);
                return;
            }
        };
        inst_builder.enable_debug_report(
            core::DebugReportType::Information | core::DebugReportType::Warning |
                core::DebugReportType::PerformanceWarning |
                core::DebugReportType::Error,
            gfx::debug::report::TermStdoutDebugReportHandler::new(),
        );
        inst_builder.enable_validation();
        inst_builder.enable_debug_marker();
        let instance: K::Instance = match inst_builder.build() {
            Ok(i) => i,
            Err(e) => {
                println!("    InstanceBuilder::build() failed: {:?}", e);
                return;
            }
        };
        for (i, adap) in instance.adapters().iter().enumerate() {
            println!("    Adapter #{}: {}", i, adap.name());

            let device_builder = instance.new_device_builder(adap);
            match device_builder.build() {
                Ok(d) => {
                    let cap = d.capabilities();
                    let limits = format!("{:#?}", cap.limits());
                    println!("        {}", limits.replace("\n", "\n        "));
                    for &fmt in core::ImageFormat::values() {
                        let f_opt = cap.image_format_features(fmt, core::ImageTiling::Optimal);
                        let f_lin = cap.image_format_features(fmt, core::ImageTiling::Linear);
                        if (f_opt | f_lin).is_empty() {
                            println!("        {:?}: not supported", fmt);
                        } else {
                            println!("        {:?}:", fmt);
                            println!("            Optimal: {:?}", f_opt);
                            println!("            Linear: {:?}", f_lin);
                        }
                    }
                    for &fmt in core::VertexFormat::values() {
                        let f = cap.vertex_format_features(fmt);
                        if f.is_empty() {
                            println!("        {:?}: not supported", fmt);
                        } else {
                            println!("        {:?}: {:?}", fmt, f);
                        }
                    }
                }
                Err(e) => {
                    println!("    DeviceBuilder::build() failed: {:?}", e);
                }
            };
            println!("");
        }
    })
}

#[cfg(target_os = "macos")]
fn try_device_metal() {
    try_environment::<gfx::backends::metal::Environment>();
}

#[cfg(not(target_os = "macos"))]
fn try_device_metal() {
    println!("    Not enabled");
}

fn try_device_vulkan() {
    try_environment::<gfx::backends::vulkan::ManagedEnvironment>();
}

fn main() {
    println!("Metal:");
    try_device_metal();
    println!("Vulkan:");
    try_device_vulkan();
}
