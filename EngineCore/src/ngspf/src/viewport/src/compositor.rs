//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use cgmath::{prelude::*, Matrix4, Vector2, Vector3, Vector4};
use refeq::RefEqArc;
use xdispatch;
use zangfx::{base as gfx, base::Result, prelude::*, utils as gfxut};

use core::{prelude::*, NodeRef, PresenterFrame};

use canvas::{ImageFormat, ImageRef};
use imagemanager::{ImageManager, ImageRefTable};
use layer::Layer;
use port::{GfxObjects, Port, PortManager};
use portrender::PortRenderFrame;
use temprespool::{TempResPool, TempResTable};
use wsi;

/// Compositor.
///
/// # Notes Regarding Memory Management
///
/// `Compositor` does not free device allocations when dropped.
#[derive(Debug)]
pub struct Compositor {
    device: gfx::DeviceRef,
    main_queue: gfx::CmdQueueRef,
    statesets: Vec<Stateset>,
    shaders: CompositorShaders,
    port_dispatch_queue: xdispatch::Queue,

    temp_res_pool: TempResPool,
    image_manager: ImageManager,

    box_vertices: gfx::BufferRef,

    white_image: gfx::ImageRef,

    sampler_repeat: gfx::SamplerRef,
    sampler_clamp: gfx::SamplerRef,

    buffer_memory_type: gfx::MemoryType,
    backing_store_memory_type: gfx::MemoryType,

    /// A clone of some GFX objects.
    gfx_objects: GfxObjects,
}

#[derive(Debug)]
struct CompositorShaders {
    composite_arg_table_sigs: [gfx::ArgTableSigRef; 2],
    composite_root_sig: gfx::RootSigRef,
    composite_library_frag: gfx::LibraryRef,
    composite_library_vert: gfx::LibraryRef,
}

static BOX_VERTICES: &[[u16; 2]] = &[[0, 0], [1, 0], [0, 1], [1, 1]];

mod composite {
    use cgmath::{Matrix4, Vector4};
    use include_data;
    use ngsenumflags::BitFlags;
    use zangfx::base::*;

    pub static SPIRV_FRAG: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/composite.frag.spv"));
    pub static SPIRV_VERT: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/composite.vert.spv"));

    // Vertex attribute locations
    pub static VA_POSITION: VertexAttrIndex = 0;

    // Argument tables
    pub static ARG_TABLE_GLOBAL: ArgTableIndex = 0;
    pub static ARG_TABLE_CONTENTS: ArgTableIndex = 1;

    // Arguments
    pub static ARG_G_SPRITE_PARAMS: ArgIndex = 0;

    pub static ARG_C_IMAGE: ArgIndex = 0;
    pub static ARG_C_IMAGE_SAMPLER: ArgIndex = 1;
    pub static ARG_C_MASK: ArgIndex = 2;
    pub static ARG_C_MASK_SAMPLER: ArgIndex = 3;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
    #[repr(u32)]
    pub enum SpriteFlagsBit {
        StraightAlpha = 1,
    }

    pub type SpriteFlags = BitFlags<SpriteFlagsBit>;

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct Sprite {
        pub matrix: Matrix4<f32>,
        pub uv_matrix: Matrix4<f32>,
        pub color: Vector4<f32>,
        pub flags: SpriteFlags,
        pub _pad: [u32; 3],
    }
}

/// Pipeline states etc. specific to a framebuffer image format.
#[derive(Debug)]
struct Stateset {
    framebuffer_format: gfx::ImageFormat,
    render_passes: Vec<gfx::RenderPassRef>,

    composite_pipeline: gfx::RenderPipelineRef,
}

const RENDER_PASS_BIT_CLEAR: usize = 1 << 0;

#[derive(Debug)]
pub struct CompositorWindow {
    compositor: Rc<RefCell<Compositor>>,

    port_manager: PortManager,

    frames: VecDeque<CompositeFrame>,

    // used as a hint to pre-allocate `Vec`s in `LocalContext`
    num_sprites: usize,
    num_contents: usize,
    num_cmds: usize,
    num_rts: usize,
}

#[derive(Debug)]
pub struct CompositeFrame {
    cb_state_tracker: gfxut::CbStateTracker,
    temp_res_table: TempResTable,
    image_ref_table: ImageRefTable,
    arg_pool: gfx::ArgPoolRef,
}

#[derive(Debug)]
pub struct CompositeContext {
    pub schedule_next_frame: bool,
    pub pixel_ratio: f32,
}

impl Compositor {
    pub fn new(gfx_objects: &GfxObjects) -> Result<Self> {
        let device = gfx_objects.device.clone();
        let main_queue = gfx_objects.main_queue.queue.clone();

        let temp_res_pool = TempResPool::new(device.clone())?;
        let mut image_manager = ImageManager::new(&device, &main_queue)?;

        let composite_arg_table_sigs = [
            {
                let mut builder = device.build_arg_table_sig();
                builder
                    .arg(composite::ARG_G_SPRITE_PARAMS, gfx::ArgType::StorageBuffer)
                    .set_stages(flags![gfx::ShaderStage::{Vertex}]);
                builder.build()?
            },
            {
                let mut builder = device.build_arg_table_sig();
                builder
                    .arg(composite::ARG_C_IMAGE, gfx::ArgType::SampledImage)
                    .set_stages(flags![gfx::ShaderStage::{Fragment}]);
                builder
                    .arg(composite::ARG_C_IMAGE_SAMPLER, gfx::ArgType::Sampler)
                    .set_stages(flags![gfx::ShaderStage::{Fragment}]);
                builder
                    .arg(composite::ARG_C_MASK, gfx::ArgType::SampledImage)
                    .set_stages(flags![gfx::ShaderStage::{Fragment}]);
                builder
                    .arg(composite::ARG_C_MASK_SAMPLER, gfx::ArgType::Sampler)
                    .set_stages(flags![gfx::ShaderStage::{Fragment}]);
                builder.build()?
            },
        ];

        let composite_root_sig = device
            .build_root_sig()
            .arg_table(0, &composite_arg_table_sigs[0])
            .arg_table(1, &composite_arg_table_sigs[1])
            .build()?;

        let composite_library_vert = device.new_library(composite::SPIRV_VERT.as_u32_slice())?;
        let composite_library_frag = device.new_library(composite::SPIRV_FRAG.as_u32_slice())?;

        let shaders = CompositorShaders {
            // "composite" shader
            composite_arg_table_sigs,
            composite_root_sig,
            composite_library_vert,
            composite_library_frag,
        };

        // Create some resources required by the compositor
        use self::gfxut::uploader::{StageBuffer, StageImage, UploaderUtils};
        let white_image = device
            .build_image()
            .extents(&[1, 1])
            .format(gfx::ImageFormat::SrgbBgra8)
            .build()?;
        {
            let memory_type = device
                .choose_memory_type(
                    white_image.get_memory_req()?.memory_types,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )
                .unwrap();

            if !device.global_heap(memory_type).bind((&white_image).into())? {
                return Err(gfx::ErrorKind::OutOfDeviceMemory.into());
            }

            let uploader = image_manager.uploader_mut();
            uploader.stage_images(
                [StageImage::new_default(
                    &white_image,
                    &[0xffffffffu32],
                    &[1, 1],
                )].iter()
                    .cloned(),
            )?;
        }

        use std::mem::size_of_val;
        let box_vertices = device
            .build_buffer()
            .size(size_of_val(BOX_VERTICES) as u64)
            .usage(flags![gfx::BufferUsage::{Vertex | CopyWrite}])
            .build()?;
        {
            let memory_type = device
                .choose_memory_type(
                    box_vertices.get_memory_req()?.memory_types,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )
                .unwrap();

            if !device
                .global_heap(memory_type)
                .bind((&box_vertices).into())?
            {
                return Err(gfx::ErrorKind::OutOfDeviceMemory.into());
            }

            let uploader = image_manager.uploader_mut();
            uploader.upload(
                [StageBuffer::new(&box_vertices, 0, BOX_VERTICES)]
                    .iter()
                    .cloned(),
            )?;
        }

        let sampler_repeat = device.build_sampler().build()?;

        let sampler_clamp = device
            .build_sampler()
            .address_mode(&[gfx::AddressMode::ClampToEdge])
            .build()?;

        // Make sure all resources are staged
        main_queue.flush();
        image_manager.uploader_mut().wait()?;

        let gfx_objects = gfx_objects.clone();

        let port_dispatch_queue = xdispatch::Queue::create(
            "com.Nightingales.NgsPF.Port",
            xdispatch::QueueAttribute::Serial,
        );

        Ok(Self {
            statesets: vec![Stateset::new(
                &*device,
                &shaders,
                gfx::ImageFormat::SrgbBgra8,
            )?],
            shaders,
            image_manager,
            temp_res_pool,
            port_dispatch_queue,

            box_vertices,

            white_image,

            sampler_repeat,
            sampler_clamp,

            buffer_memory_type: device
                .try_choose_memory_type_shared(flags![gfx::BufferUsage::{Storage}])?
                .unwrap(),
            backing_store_memory_type: device
                .try_choose_memory_type_private(gfx::ImageFormat::SrgbBgra8)?
                .unwrap(),

            device,
            main_queue,
            gfx_objects,
        })
    }

    fn retire_frame(&mut self, mut frame: CompositeFrame) -> Result<()> {
        assert!(frame.cb_state_tracker.is_completed());
        self.temp_res_pool.release(&mut frame.temp_res_table)?;
        self.image_manager.release(&mut frame.image_ref_table)?;
        Ok(())
    }
}

impl Drop for CompositorWindow {
    fn drop(&mut self) {
        let mut compositor = self.compositor.borrow_mut();
        for frame in self.frames.drain(..) {
            frame.cb_state_tracker.wait();
            compositor.retire_frame(frame).unwrap();
        }
    }
}

impl CompositorWindow {
    pub fn new(compositor: Rc<RefCell<Compositor>>) -> Result<Self> {
        Ok(Self {
            port_manager: PortManager::new(),

            compositor,

            frames: VecDeque::new(),

            num_sprites: 0,
            num_contents: 0,
            num_cmds: 0,
            num_rts: 0,
        })
    }

    pub fn composite(
        &mut self,
        context: &mut CompositeContext,
        root: &Option<NodeRef>,
        frame: &PresenterFrame,
        drawable: &mut wsi::Drawable,
    ) -> Result<()> {
        self.frames.reserve(1);

        use std::mem::size_of_val;

        use cggeom::prelude::*;
        use cggeom::Box2;

        enum Cmd {
            BeginPass {
                pass_i: usize,
                rt_i: usize,
            },
            EndPass,
            EndPassForPresentation,
            Sprite {
                instance_i: usize,
                contents_i: usize,
                count: usize,
            },
        }

        #[derive(Debug, Clone)]
        enum ImageContents {
            Image(gfx::ImageRef),
            ManagedImage(ImageRef),
            Port(RefEqArc<Port>),
        }

        impl From<gfx::ImageRef> for ImageContents {
            fn from(x: gfx::ImageRef) -> Self {
                ImageContents::Image(x)
            }
        }

        struct LocalContext<'a> {
            compositor: &'a mut Compositor,
            frame: &'a PresenterFrame,

            sprites: Vec<composite::Sprite>,
            contents: Vec<[(ImageContents, gfx::SamplerRef); 2]>,
            cmds: Vec<Vec<Cmd>>,
            rts: Vec<RenderTarget>,

            temp_res_table: TempResTable,
            image_ref_table: ImageRefTable,
        }

        struct RenderTarget {
            image: gfx::ImageRef,
            extents: Vector2<u32>,
        }

        struct RasterContext<'a> {
            cmd_group_i: usize,
            begin_pass_cmd_i: usize,
            image: &'a gfx::ImageRef,
        }

        struct BackDropInfo {
            image: gfx::ImageRef,
            uv_matrix: Matrix4<f32>,
        }

        fn model_mat_for_bounds(bounds: &Box2<f32>) -> Matrix4<f32> {
            let size = bounds.size();
            Matrix4::from_translation(bounds.min.to_vec().extend(0.0))
                * Matrix4::from_nonuniform_scale(size.x, size.y, 1.0)
        }

        fn render_inner(
            cc: &mut CompositeContext,
            c: &mut LocalContext,
            rc: &mut RasterContext,
            layer: &Layer,
            matrix: Matrix4<f32>,
            opacity: f32,
            backdrop: Option<BackDropInfo>,
        ) -> Result<()> {
            use super::ImageWrapMode::*;
            use super::LayerContents::*;

            let contents = layer.contents.read_presenter(c.frame).unwrap();
            let bounds: Box2<f32> = *layer.bounds.read_presenter(c.frame).unwrap();
            let model_matrix = matrix * model_mat_for_bounds(&bounds);

            let sprite_info = match contents {
                &Empty => None,
                &Image {
                    ref image,
                    ref source,
                    ref wrap_mode,
                } => {
                    let sampler = match *wrap_mode {
                        Repeat => c.compositor.sampler_repeat.clone(),
                        Clamp => c.compositor.sampler_clamp.clone(),
                    };
                    let image_data = image.image_data();
                    let image_data = image_data.get_presenter_ref(c.frame).unwrap();

                    let size = image_data.size();
                    let size_f = size.cast::<f32>().unwrap();
                    let uv_matrix =
                        Matrix4::from_nonuniform_scale(1.0 / size_f.x, 1.0 / size_f.y, 1.0)
                            * model_mat_for_bounds(source);

                    let premul = match image_data.format() {
                        ImageFormat::SrgbRgba8 => false,
                        ImageFormat::SrgbRgba8Premul => true,
                    };

                    c.compositor
                        .image_manager
                        .use_image(image, &mut c.image_ref_table);

                    Some((
                        (ImageContents::ManagedImage(image.clone()), sampler),
                        uv_matrix,
                        if premul {
                            flags![composite::SpriteFlagsBit::{}]
                        } else {
                            flags![composite::SpriteFlagsBit::{StraightAlpha}]
                        },
                        Vector4::new(1.0, 1.0, 1.0, opacity),
                    ))
                }
                &Solid(rgba) => Some((
                    (
                        c.compositor.white_image.clone().into(),
                        c.compositor.sampler_clamp.clone(),
                    ),
                    Matrix4::identity(),
                    composite::SpriteFlags::empty(),
                    Vector4::new(rgba.r, rgba.g, rgba.b, 1.0) * (opacity * rgba.a),
                )),
                &Port(ref port) => Some((
                    (
                        ImageContents::Port(port.clone()),
                        c.compositor.sampler_clamp.clone(),
                    ),
                    Matrix4::identity(),
                    composite::SpriteFlags::empty(),
                    Vector4::new(1.0, 1.0, 1.0, opacity),
                )),
                &BackDrop => {
                    let backdrop = backdrop.expect("BackDrop used without FlattenContents");
                    Some((
                        (backdrop.image.into(), c.compositor.sampler_clamp.clone()),
                        backdrop.uv_matrix,
                        composite::SpriteFlags::empty(),
                        Vector4::new(1.0, 1.0, 1.0, opacity),
                    ))
                }
            };

            if let Some((image_contents, uv_matrix, flags, color)) = sprite_info {
                let instance_i = c.sprites.len();
                let contents_i = c.contents.len();
                c.contents.push([
                    image_contents,
                    (
                        c.compositor.white_image.clone().into(),
                        c.compositor.sampler_clamp.clone(),
                    ),
                ]);
                c.sprites.push(composite::Sprite {
                    matrix: model_matrix,
                    uv_matrix,
                    color,
                    flags,
                    _pad: [0; 3],
                });
                c.cmds[rc.cmd_group_i].push(Cmd::Sprite {
                    instance_i,
                    contents_i,
                    count: 1,
                });
            }

            if let &Some(ref child) = layer.child.read_presenter(c.frame).unwrap() {
                child.for_each_node_of_r(|layer: &Layer| {
                    traverse(cc, c, rc, layer, matrix, opacity)
                })?;
            }

            Ok(())
        }

        fn traverse(
            cc: &mut CompositeContext,
            c: &mut LocalContext,
            rc: &mut RasterContext,
            layer: &Layer,
            matrix: Matrix4<f32>,
            opacity: f32,
        ) -> Result<()> {
            use super::LayerContents::*;
            use super::{LayerFlags, LayerFlagsBit};

            let flags: LayerFlags = *layer.flags.read_presenter(c.frame).unwrap();
            let transform = *layer.transform.read_presenter(c.frame).unwrap();
            let contents = layer.contents.read_presenter(c.frame).unwrap();
            let mask = layer.mask.read_presenter(c.frame).unwrap();
            let bounds: Box2<f32> = *layer.bounds.read_presenter(c.frame).unwrap();
            let opacity = opacity * *layer.opacity.read_presenter(c.frame).unwrap();

            let flatten = flags.contains(LayerFlagsBit::FlattenContents);
            let use_backdrop = match contents {
                &BackDrop => true,
                _ => false,
            };

            let local_matrix = matrix * transform;

            if flatten {
                if bounds.is_empty() {
                    return Ok(());
                }

                let model_matrix = local_matrix * model_mat_for_bounds(&bounds);

                // Dimensions of the flattened image
                let size = bounds.size();
                let mut pixel_size = (size * cc.pixel_ratio).cast::<u32>().unwrap();
                if pixel_size.x == 0 {
                    pixel_size.x = 1;
                }
                if pixel_size.y == 0 {
                    pixel_size.y = 1;
                }

                // Transformation matrix from the inner contents to the clip
                // space
                let inner_matrix = Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.5))
                    * Matrix4::from_nonuniform_scale(2.0 / size.x, 2.0 / size.y, 0.0)
                    * Matrix4::from_translation(-bounds.min.to_vec().extend(0.0));

                let mut saved = None;
                let cmd_group_i;
                let backdrop;
                if use_backdrop {
                    // We need the rendered contents of the parent raster context.
                    // Interrupt the parent raster context's render pass and
                    // encode commands to the parent raster context's CB
                    match c.cmds[rc.cmd_group_i][rc.begin_pass_cmd_i] {
                        Cmd::BeginPass {
                            ref mut pass_i,
                            ref mut rt_i,
                        } => {
                            saved = Some((*pass_i, *rt_i));
                        }
                        _ => unreachable!(),
                    }
                    c.cmds[rc.cmd_group_i].push(Cmd::EndPass);
                    cmd_group_i = rc.cmd_group_i;

                    backdrop = Some(BackDropInfo {
                        image: rc.image.clone(),
                        uv_matrix: Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.0))
                            * Matrix4::from_nonuniform_scale(0.5, 0.5, 1.0)
                            * model_matrix,
                    });
                } else {
                    // Create a new CB that are scheduled before the parent
                    // raster context's CB.
                    c.cmds.push(Vec::new());
                    cmd_group_i = c.cmds.len() - 1;
                    backdrop = None;
                }

                // Create a backing store image
                let image = c
                    .compositor
                    .device
                    .build_image()
                    .extents(&pixel_size[..])
                    .format(gfx::ImageFormat::SrgbRgba8)
                    .usage(flags![gfx::ImageUsage::{Render | Sampled}])
                    .build()?;

                c.compositor.temp_res_pool.bind(
                    &mut c.temp_res_table,
                    c.compositor.backing_store_memory_type,
                    &image,
                )?;

                c.rts.push(RenderTarget {
                    image: image.clone(),
                    extents: pixel_size,
                });

                let rt_i = c.rts.len() - 1;

                c.cmds[cmd_group_i].push(Cmd::BeginPass {
                    pass_i: RENDER_PASS_BIT_CLEAR,
                    rt_i,
                });

                // Render the contents and children
                {
                    let mut new_rc = RasterContext {
                        cmd_group_i,
                        begin_pass_cmd_i: c.cmds[cmd_group_i].len() - 1,
                        image: &image,
                    };
                    render_inner(cc, c, &mut new_rc, layer, inner_matrix, 1.0, backdrop)?;
                }

                c.cmds[cmd_group_i].push(Cmd::EndPass);

                if use_backdrop {
                    let (pass_i, rt_i) = saved.unwrap();
                    // Restart the interrupted render pass
                    c.cmds[rc.cmd_group_i].push(Cmd::BeginPass {
                        pass_i: pass_i & !RENDER_PASS_BIT_CLEAR,
                        rt_i,
                    });
                    rc.begin_pass_cmd_i = c.cmds[rc.cmd_group_i].len() - 1;
                }

                // Render the mask image
                let mask_contents = if let &Some(ref mask) = mask {
                    // Create a mask image
                    let mask_image = c
                        .compositor
                        .device
                        .build_image()
                        .extents(&pixel_size[..])
                        .format(gfx::ImageFormat::SrgbBgra8)
                        .usage(flags![gfx::ImageUsage::{Render | Sampled}])
                        .build()?;

                    c.compositor.temp_res_pool.bind(
                        &mut c.temp_res_table,
                        c.compositor.backing_store_memory_type,
                        &mask_image,
                    )?;

                    c.rts.push(RenderTarget {
                        image: mask_image.clone(),
                        extents: pixel_size,
                    });

                    let mask_rt_i = c.rts.len() - 1;

                    c.cmds.push(vec![Cmd::BeginPass {
                        pass_i: RENDER_PASS_BIT_CLEAR,
                        rt_i: mask_rt_i,
                    }]);
                    let mask_cmd_group_i = c.cmds.len() - 1;

                    {
                        let mut mask_rc = RasterContext {
                            cmd_group_i: mask_cmd_group_i,
                            begin_pass_cmd_i: 0,
                            image: &mask_image,
                        };

                        mask.for_each_node_of_r(|layer: &Layer| {
                            traverse(cc, c, &mut mask_rc, layer, inner_matrix, 1.0)
                        })?;
                    }

                    c.cmds[mask_cmd_group_i].push(Cmd::EndPass);

                    (mask_image.into(), c.compositor.sampler_clamp.clone())
                } else {
                    (
                        c.compositor.white_image.clone().into(),
                        c.compositor.sampler_clamp.clone(),
                    )
                };

                // Now composite the flattened contents to the parent raster
                // context's image
                let instance_i = c.sprites.len();
                let contents_i = c.contents.len();
                c.contents.push([
                    (image.into(), c.compositor.sampler_clamp.clone()),
                    mask_contents,
                ]);
                c.sprites.push(composite::Sprite {
                    matrix: model_matrix,
                    uv_matrix: Matrix4::identity(),
                    color: Vector4::new(1.0, 1.0, 1.0, opacity),
                    flags: composite::SpriteFlags::empty(),
                    _pad: [0; 3],
                });
                c.cmds[rc.cmd_group_i].push(Cmd::Sprite {
                    instance_i,
                    contents_i,
                    count: 1,
                });
            } else {
                render_inner(cc, c, rc, layer, local_matrix, opacity, None)?;
            }

            Ok(())
        }

        let mut compositor = self.compositor.borrow_mut();
        let ref mut compositor = *compositor; // Enable partial borrows

        let surface_props = drawable.surface_props().clone();
        let dpi_width = surface_props.extents[0] as f32 / context.pixel_ratio;
        let dpi_height = surface_props.extents[1] as f32 / context.pixel_ratio;

        let mut temp_res_table = compositor.temp_res_pool.new_table();
        let image_ref_table = compositor.image_manager.new_ref_table();

        // Scan for `Port`s first
        self.port_manager.prepare_frame();

        let port_frame = PortRenderFrame::new(
            &mut compositor.port_dispatch_queue,
            frame,
            root,
            &compositor.gfx_objects,
            &mut compositor.temp_res_pool,
            &mut temp_res_table,
            compositor.backing_store_memory_type,
            &mut self.port_manager,
        )?;

        let mut c = LocalContext {
            compositor: &mut *compositor,
            frame,
            temp_res_table,
            image_ref_table,
            sprites: Vec::with_capacity(self.num_sprites * 2),
            contents: Vec::with_capacity(self.num_contents * 2),
            cmds: Vec::with_capacity(self.num_cmds * 2),
            rts: Vec::with_capacity(self.num_rts * 2),
        };
        c.cmds.push(vec![Cmd::BeginPass {
            pass_i: RENDER_PASS_BIT_CLEAR,
            rt_i: 0,
        }]);
        c.rts.push(RenderTarget {
            image: drawable.image().clone(),
            extents: Vector2::from(surface_props.extents),
        });
        if let &Some(ref root) = root {
            let root_matrix = Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.5))
                * Matrix4::from_nonuniform_scale(2.0 / dpi_width, 2.0 / dpi_height, 0.0);

            let mut rc = RasterContext {
                cmd_group_i: 0,
                begin_pass_cmd_i: 0,
                image: drawable.image(),
            };

            root.for_each_node_of_r(|layer: &Layer| {
                traverse(context, &mut c, &mut rc, layer, root_matrix, 1.0)
            })?;
        }
        c.cmds[0].push(Cmd::EndPassForPresentation);

        self.num_sprites = c.sprites.len();
        self.num_contents = c.contents.len();
        self.num_cmds = c.cmds.len();
        self.num_rts = c.rts.len();

        // Collect various data
        struct RtData {
            viewport: gfx::Viewport,
            framebuffer: [Option<gfx::RenderTargetTableRef>; 6],
            rt: RenderTarget,
        }

        let ref mut compositor = c.compositor;

        let mut rt_data: Vec<_> = c
            .rts
            .into_iter()
            .map(|rt| RtData {
                viewport: gfx::Viewport {
                    x: 0f32,
                    y: 0f32,
                    width: rt.extents.x as f32,
                    height: rt.extents.y as f32,
                    min_depth: 0f32,
                    max_depth: 1f32,
                },
                framebuffer: Default::default(),
                rt,
            })
            .collect();

        // Prepare to upload `Sprite`
        let sprites_size = size_of_val(c.sprites.as_slice()) as gfx::DeviceSize;
        let sprites_buf = compositor
            .device
            .build_buffer()
            .size(sprites_size)
            .usage(flags![gfx::BufferUsage::{Storage}])
            .build()?;
        compositor.temp_res_pool.bind(
            &mut c.temp_res_table,
            compositor.buffer_memory_type,
            &sprites_buf,
        )?;
        {
            use std::slice::from_raw_parts_mut;
            let sprites_slice = unsafe {
                from_raw_parts_mut(
                    sprites_buf.as_ptr() as *mut composite::Sprite,
                    c.sprites.len(),
                )
            };
            sprites_slice.copy_from_slice(c.sprites.as_slice());
        }

        // Initiate the upload of images.
        let image_session_id = compositor.image_manager.upload(frame)?;
        let mut fence = compositor
            .image_manager
            .get_fence_for_session(image_session_id)
            .cloned();

        // Resolve all image view references
        let contents_images: Vec<[gfx::ImageRef; 2]> = c
            .contents
            .iter()
            .map(|contents| {
                [
                    match contents[0].0 {
                        ImageContents::Image(ref image) => image.clone(),
                        ImageContents::ManagedImage(ref image_ref) => {
                            let resident_image = compositor.image_manager.get(&image_ref).unwrap();
                            resident_image.image().clone()
                        }
                        ImageContents::Port(ref port) => {
                            let port_output = port_frame.get_output(port).unwrap();
                            port_output.image.clone()
                        }
                    },
                    match contents[1].0 {
                        ImageContents::Image(ref image) => image.clone(),
                        _ => unreachable!(),
                    },
                ]
            })
            .collect();

        // Make argument tables
        let arg_pool;
        let at_global;
        let at_contents;
        {
            let ref shaders = compositor.shaders;
            arg_pool = compositor
                .device
                .build_arg_pool()
                .reserve_table_sig(
                    c.contents.len(),
                    &shaders.composite_arg_table_sigs[composite::ARG_TABLE_CONTENTS],
                )
                .reserve_table_sig(
                    1,
                    &shaders.composite_arg_table_sigs[composite::ARG_TABLE_GLOBAL],
                )
                .build()?;

            at_global = arg_pool
                .new_table(&shaders.composite_arg_table_sigs[composite::ARG_TABLE_GLOBAL])?
                .unwrap();

            compositor.device.update_arg_table(
                &shaders.composite_arg_table_sigs[composite::ARG_TABLE_GLOBAL],
                &arg_pool,
                &at_global,
                &[(
                    composite::ARG_G_SPRITE_PARAMS,
                    0,
                    [(0..sprites_size, &sprites_buf)][..].into(),
                )],
            )?;

            at_contents = arg_pool
                .new_tables(
                    c.contents.len(),
                    &shaders.composite_arg_table_sigs[composite::ARG_TABLE_CONTENTS],
                )?
                .unwrap();

            let mut at_contents_images = Vec::with_capacity(c.contents.len() * 2);
            let mut at_contents_samplers = Vec::with_capacity(c.contents.len() * 2);

            for (contents, images) in c.contents.iter().zip(contents_images.iter()) {
                for (&(_, ref sampler), image) in contents[0..2].iter().zip(images.iter()) {
                    at_contents_images.push(image);
                    at_contents_samplers.push(sampler);
                }
            }

            let mut at_contents_update_sets = Vec::with_capacity(c.contents.len() * 4);

            for i in 0..c.contents.len() {
                at_contents_update_sets.push((
                    composite::ARG_C_IMAGE,
                    0,
                    (&at_contents_images[i * 2..][..1]).into(),
                ));
                at_contents_update_sets.push((
                    composite::ARG_C_MASK,
                    0,
                    (&at_contents_images[i * 2 + 1..][..1]).into(),
                ));
                at_contents_update_sets.push((
                    composite::ARG_C_IMAGE_SAMPLER,
                    0,
                    (&at_contents_samplers[i * 2..][..1]).into(),
                ));
                at_contents_update_sets.push((
                    composite::ARG_C_MASK_SAMPLER,
                    0,
                    (&at_contents_samplers[i * 2 + 1..][..1]).into(),
                ));
            }

            let at_contents_updates: Vec<_> = at_contents_update_sets
                .chunks(4)
                .zip(at_contents.iter())
                .map(|(update_sets, arg_table)| ((&arg_pool, arg_table), update_sets))
                .collect();

            compositor.device.update_arg_tables(
                &shaders.composite_arg_table_sigs[composite::ARG_TABLE_CONTENTS],
                &at_contents_updates[..],
            )?;
        }

        // Retire old frames
        while self.frames.len() > 0 {
            if !self.frames[0].cb_state_tracker.is_completed() {
                if self.frames.len() > 2 {
                    self.frames[0].cb_state_tracker.wait();
                } else {
                    break;
                }
            }

            compositor.retire_frame(self.frames.pop_front().unwrap())?;
        }

        // Create an execution barrier

        // Encode the command buffer
        let mut cb = compositor.main_queue.new_cmd_buffer()?;
        let cb_state_tracker = gfxut::CbStateTracker::new(&mut *cb);
        {
            let mut it = c.cmds.iter().rev().flat_map(|cmds| cmds.iter());
            while let Some(cmd) = it.next() {
                let enc;
                if let &Cmd::BeginPass { pass_i, rt_i } = cmd {
                    {
                        let ref mut rt_data = rt_data[rt_i];
                        let ref mut fb = rt_data.framebuffer[pass_i];
                        if fb.is_none() {
                            let mut builder = compositor.device.build_render_target_table();
                            builder
                                .render_pass(&compositor.statesets[0].render_passes[pass_i])
                                .extents(&rt_data.rt.extents[..]);
                            builder.target(0, &rt_data.rt.image).clear_float(&[0f32; 4]);
                            *fb = Some(builder.build()?);
                        }
                    }

                    let fb = rt_data[rt_i].framebuffer[pass_i].as_ref().unwrap();
                    enc = cb.encode_render(fb);

                    if let Some(ref fence) = fence.take() {
                        enc.wait_fence(fence, flags![gfx::AccessType::{FragmentRead}]);
                    }

                    enc.bind_pipeline(&compositor.statesets[0].composite_pipeline);
                    enc.bind_vertex_buffers(0, &[(&compositor.box_vertices, 0)]);
                    enc.set_viewports(0, &[rt_data[rt_i].viewport]);
                    enc.bind_arg_table(composite::ARG_TABLE_GLOBAL, &[(&arg_pool, &at_global)]);

                    enc.use_resource_read(&sprites_buf);
                } else {
                    unreachable!();
                }

                // TODO: insert fences *between* render passes

                loop {
                    match it.next().unwrap() {
                        &Cmd::BeginPass { .. } => unreachable!(),
                        &Cmd::EndPass => {
                            break;
                        }
                        &Cmd::EndPassForPresentation => {
                            break;
                        }
                        &Cmd::Sprite {
                            instance_i,
                            contents_i,
                            count,
                        } => {
                            // If the image source is a `Port`, then insert a fence and image layout transition
                            match c.contents[contents_i][0].0 {
                                ImageContents::Port(ref port) => {
                                    let port_output = port_frame.get_output(port).unwrap();

                                    enc.wait_fence(
                                        &port_output.fence,
                                        flags![gfx::AccessType::{FragmentRead}],
                                    );
                                }
                                _ => {}
                            }
                            enc.use_resource_read(
                                &[
                                    &contents_images[contents_i][0],
                                    &contents_images[contents_i][1],
                                ][..],
                            );
                            enc.bind_arg_table(
                                composite::ARG_TABLE_CONTENTS,
                                &[(&arg_pool, &at_contents[contents_i])],
                            );
                            let instance_i = instance_i as u32;
                            let count = count as u32;
                            enc.draw(0..4, instance_i..instance_i + count);
                        }
                    }
                }
            }
        }

        drawable.encode_prepare_present(
            &mut cb,
            compositor.gfx_objects.main_queue.queue_family,
            flags![gfx::Stage::{RenderOutput}],
            flags![gfx::AccessType::{ColorWrite}],
        );

        cb.commit()?;

        // Make sure ports' CBs are commited too
        drop(port_frame);

        compositor.main_queue.flush();

        self.frames.push_back(CompositeFrame {
            temp_res_table: c.temp_res_table,
            image_ref_table: c.image_ref_table,
            arg_pool,
            cb_state_tracker,
        });

        drawable.enqueue_present();

        Ok(())
    }
}

impl Stateset {
    fn new(
        device: &gfx::Device,
        shaders: &CompositorShaders,
        framebuffer_format: gfx::ImageFormat,
    ) -> Result<Self> {
        let render_passes: Vec<_> = (0..2)
            .map(|i| {
                let mut builder = device.build_render_pass();
                builder.label("Compositor render pass");

                builder
                    .target(0)
                    .set_format(framebuffer_format)
                    .set_load_op(if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                        gfx::LoadOp::Clear
                    } else {
                        gfx::LoadOp::Load
                    })
                    .set_store_op(gfx::StoreOp::Store);

                builder.subpass_color_targets(&[Some(0)]);

                builder.build()
            })
            .collect::<Result<_>>()?;

        let composite_pipeline = {
            let mut builder = device.build_render_pipeline();
            builder
                .label("Composite")
                .vertex_shader(&shaders.composite_library_vert, "main")
                .fragment_shader(&shaders.composite_library_frag, "main")
                .root_sig(&shaders.composite_root_sig)
                .topology(gfx::PrimitiveTopology::TriangleStrip)
                .render_pass(&render_passes[0], 0);

            builder.vertex_buffer(0, 4 /* stride */);
            builder.vertex_attr(composite::VA_POSITION, 0, 0, <u16>::as_format_unnorm() * 2);

            builder
                .rasterize()
                .color_target(0)
                .set_blending(true)
                .set_src_alpha_factor(gfx::BlendFactor::One)
                .set_src_rgb_factor(gfx::BlendFactor::One)
                .set_dst_alpha_factor(gfx::BlendFactor::OneMinusSrcAlpha)
                .set_dst_rgb_factor(gfx::BlendFactor::OneMinusSrcAlpha)
                .set_alpha_op(gfx::BlendOp::Add)
                .set_rgb_op(gfx::BlendOp::Add);
            builder.build()?
        };

        Ok(Self {
            framebuffer_format,
            render_passes,
            composite_pipeline,
        })
    }
}
