//! NgsGFX − Nightingales Graphics Backend
//! ======================================
//!
//! Abstracts the interface to the graphics APIs (e.g., Vulkan, Metal).
//!
//! See the [`ngsgfx_core`][core] crate's documentation for the usage.
//!
//! [core]: ../ngsgfx_core/index.html

extern crate ngsgfx_core;
extern crate ngsgfx_metal;

pub use ::ngsgfx_core::*;

pub mod backends {
    pub mod metal {
        pub use ::ngsgfx_metal::*;
    }
}

/// Contains frequently used traits for convenience.
pub mod prelude {
    pub use ::Buffer;
    pub use ::BufferView;
    pub use ::{CommandBuffer, CommandEncoder};
    pub use ::CommandQueue;
    pub use ::ComputePipeline;
    pub use ::DescriptorPool;
    pub use ::DescriptorSet;
    pub use ::DescriptorSetLayout;
    pub use ::Device;
    pub use ::DeviceCapabilities;
    pub use ::Factory;
    pub use ::Fence;
    pub use ::Framebuffer;
    pub use ::GraphicsPipeline;
    pub use ::{Heap, MappableHeap};
    pub use ::Image;
    pub use ::ImageView;
    pub use ::PipelineLayout;
    pub use ::RenderPass;
    pub use ::Sampler;
    pub use ::Semaphore;
    pub use ::StencilState;
    pub use ::ShaderModule;
}
