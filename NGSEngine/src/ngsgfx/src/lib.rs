//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX − Nightingales Graphics Backend
//! ======================================
//!
//! Abstracts the interface to the graphics APIs (e.g., Vulkan, Metal).
//!
//! See the [`ngsgfx_core`][core] crate's documentation for the usage.
//!
//! [core]: ../ngsgfx_core/index.html

pub extern crate ngsgfx_core as core;
extern crate ngsgfx_wsi_core;

pub extern crate ngsgfx_debug as debug;

/// Includes a backend for each target API.
pub mod backends {
    #[cfg(target_os = "macos")]
    pub extern crate ngsgfx_metal as metal;

    pub extern crate ngsgfx_vulkan as vulkan;

    #[cfg(target_os = "macos")]
    pub use self::metal::Environment as DefaultEnvironment;
    #[cfg(target_os = "macos")]
    pub use self::metal::Backend as DefaultBackend;

    #[cfg(not(target_os = "macos"))]
    pub use self::vulkan::ManagedEnvironment as DefaultEnvironment;
    #[cfg(not(target_os = "macos"))]
    pub use self::vulkan::ManagedBackend as DefaultBackend;
}

/// Provides the window system integration functionality.
///
/// Provides an alias to a type implementing `NewWindow`, named `DefaultWindow`
/// that represents the default implementation on the target platform;
pub mod wsi {
    pub use ngsgfx_wsi_core::*;

    #[cfg(target_os = "macos")]
    pub extern crate ngsgfx_wsi_metal as metal;

    pub extern crate ngsgfx_wsi_vulkan as vulkan;

    #[cfg(target_os = "macos")]
    pub use self::metal::MetalWindow as DefaultWindow;

    #[cfg(any(windows, all(unix, not(target_os = "macos"), not(target_os = "android"))))]
    pub use self::vulkan::DefaultVulkanWindow as DefaultWindow;
}

/// Contains frequently used traits (from `ngsgfx_core`) for convenience.
pub mod prelude {
    pub use core::Backend;

    pub use core::Buffer;
    pub use core::{CommandBuffer, CommandEncoder};
    pub use core::{RenderSubpassCommandEncoder, ComputeCommandEncoder, CopyCommandEncoder,
                   DebugCommandEncoder, BarrierCommandEncoder};
    pub use core::SecondaryCommandBuffer;
    pub use core::CommandQueue;
    pub use core::ComputePipeline;
    pub use core::DescriptorPool;
    pub use core::DescriptorSet;
    pub use core::DescriptorSetLayout;
    pub use core::Device;
    pub use core::DeviceCapabilities;
    pub use core::Factory;
    pub use core::Event;
    pub use core::Framebuffer;
    pub use core::GraphicsPipeline;
    pub use core::{Heap, MappableHeap};
    pub use core::Image;
    pub use core::ImageView;
    pub use core::Marker;
    pub use core::PipelineLayout;
    pub use core::RenderPass;
    pub use core::Sampler;
    pub use core::StencilState;
    pub use core::ShaderModule;
}
