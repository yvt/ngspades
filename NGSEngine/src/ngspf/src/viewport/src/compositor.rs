//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

use refeq::RefEqArc;
use cgmath::{Matrix4, Vector2, Vector3, Vector4, prelude::*};
use zangfx::{base as gfx, utils as gfxut, base::Result, prelude::*};
use xdispatch;

use core::{NodeRef, PresenterFrame, prelude::*};

use layer::Layer;
use temprespool::{TempResPool, TempResTable};
use imagemanager::{ImageManager, ImageRefTable};
use port::{GfxObjects, Port, PortManager};
use portrender::PortRenderFrame;
use image::ImageRef;
use wsi;

/// Compositor.
///
/// # Notes Regarding Memory Management
///
/// `Compositor` does not free device allocations when dropped.
#[derive(Debug)]
pub struct Compositor {
    device: Arc<gfx::Device>,
    main_queue: Arc<gfx::CmdQueue>,
    statesets: Vec<Stateset>,
    shaders: CompositorShaders,
    cmd_pool: Box<gfx::CmdPool>,
    port_dispatch_queue: xdispatch::Queue,

    temp_res_pool: TempResPool,
    image_manager: ImageManager,

    box_vertices: gfxut::UniqueBuffer<Arc<gfx::Device>>,

    white_image: gfxut::UniqueImage<Arc<gfx::Device>>,
    white_image_view: gfxut::UniqueImageView<Arc<gfx::Device>>,

    sampler_repeat: gfxut::UniqueSampler<Arc<gfx::Device>>,
    sampler_clamp: gfxut::UniqueSampler<Arc<gfx::Device>>,

    buffer_memory_type: gfx::MemoryType,
    backing_store_memory_type: gfx::MemoryType,

    /// A clone of some GFX objects.
    gfx_objects: GfxObjects,
}

#[derive(Debug)]
struct CompositorShaders {
    composite_arg_table_sigs: [gfx::ArgTableSig; 2],
    composite_root_sig: gfx::RootSig,
    composite_library_frag: gfx::Library,
    composite_library_vert: gfx::Library,
}

static BOX_VERTICES: &[[u16; 2]] = &[[0, 0], [1, 0], [0, 1], [1, 1]];

mod composite {
    use include_data;
    use zangfx::base::*;
    use cgmath::{Matrix4, Vector4};
    use ngsenumflags::BitFlags;

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
    render_passes: Vec<gfx::RenderPass>,

    composite_pipeline: gfx::RenderPipeline,
}

const RENDER_PASS_BIT_CLEAR: usize = 1 << 0;
const RENDER_PASS_BIT_USAGE_MASK: usize = 0b11 << 1;
const RENDER_PASS_BIT_USAGE_PRESENT: usize = 0b00 << 1;
const RENDER_PASS_BIT_USAGE_SHADER_READ: usize = 0b01 << 1;
const RENDER_PASS_BIT_USAGE_GENERAL: usize = 0b10 << 1;

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
    arg_pool: Box<gfx::ArgPool>,
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

        let cmd_pool = gfx_objects.main_queue.queue.new_cmd_pool()?;

        let mut temp_res_pool = TempResPool::new(Arc::clone(&device))?;
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
        let white_image = gfxut::UniqueImage::new(device.clone(), white_image);
        {
            let memory_type = device
                .choose_memory_type(
                    device.get_memory_req((&*white_image).into())?.memory_types,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )
                .unwrap();
            temp_res_pool
                .heap_mut()
                .bind_dynamic(memory_type, &*white_image)?;

            let uploader = image_manager.uploader_mut();
            uploader.stage_images(
                [
                    StageImage::new_default(
                        &*white_image,
                        gfx::ImageLayout::ShaderRead,
                        &[0xffffffffu32],
                        &[1, 1],
                    ),
                ].iter()
                    .cloned(),
            )?;
        }
        let white_image_view = device.new_image_view(&*white_image, gfx::ImageLayout::ShaderRead)?;
        let white_image_view = gfxut::UniqueImageView::new(device.clone(), white_image_view);

        use std::mem::size_of_val;
        let box_vertices = device
            .build_buffer()
            .size(size_of_val(BOX_VERTICES) as u64)
            .usage(flags![gfx::BufferUsage::{Vertex}])
            .build()?;
        let box_vertices = gfxut::UniqueBuffer::new(device.clone(), box_vertices);
        {
            let memory_type = device
                .choose_memory_type(
                    device.get_memory_req((&*box_vertices).into())?.memory_types,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )
                .unwrap();
            temp_res_pool
                .heap_mut()
                .bind_dynamic(memory_type, &*box_vertices)?;

            let uploader = image_manager.uploader_mut();
            uploader.upload(
                [StageBuffer::new(&*box_vertices, 0, BOX_VERTICES)]
                    .iter()
                    .cloned(),
            )?;
        }

        let sampler_repeat = device.build_sampler().build()?;
        let sampler_repeat = gfxut::UniqueSampler::new(device.clone(), sampler_repeat);

        let sampler_clamp = device
            .build_sampler()
            .address_mode(&[gfx::AddressMode::ClampToEdge])
            .build()?;
        let sampler_clamp = gfxut::UniqueSampler::new(device.clone(), sampler_clamp);

        // Make sure all resources are staged
        main_queue.flush();
        image_manager.uploader_mut().wait()?;

        let gfx_objects = gfx_objects.clone();

        let port_dispatch_queue = xdispatch::Queue::create(
            "com.Nightingales.NgsPF.Port",
            xdispatch::QueueAttribute::Serial,
        );

        Ok(Self {
            statesets: vec![
                Stateset::new(&*device, &shaders, gfx::ImageFormat::SrgbBgra8)?,
            ],
            shaders,
            image_manager,
            temp_res_pool,
            cmd_pool,
            port_dispatch_queue,

            box_vertices,

            white_image,
            white_image_view,

            sampler_repeat,
            sampler_clamp,

            buffer_memory_type: device
                .memory_type_for_buffer(
                    flags![gfx::BufferUsage::{Storage}],
                    flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
                    flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
                )?
                .unwrap(),
            backing_store_memory_type: device
                .memory_type_for_image(
                    gfx::ImageFormat::SrgbBgra8,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )?
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

        use ngsbase::Box2;
        use ngsbase::prelude::*;

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
            Image(gfx::ImageView, gfx::Image),
            ManagedImage(ImageRef),
            Port(RefEqArc<Port>),
        }

        impl From<(gfx::ImageView, gfx::Image)> for ImageContents {
            fn from(x: (gfx::ImageView, gfx::Image)) -> Self {
                ImageContents::Image(x.0, x.1)
            }
        }

        struct LocalContext<'a> {
            compositor: &'a mut Compositor,
            frame: &'a PresenterFrame,

            sprites: Vec<composite::Sprite>,
            contents: Vec<[(ImageContents, gfx::Sampler); 2]>,
            cmds: Vec<Vec<Cmd>>,
            rts: Vec<RenderTarget>,

            temp_res_table: TempResTable,
            image_ref_table: ImageRefTable,
        }

        struct RenderTarget {
            image: gfx::Image,
            extents: Vector2<u32>,
        }

        struct RasterContext<'a> {
            cmd_group_i: usize,
            begin_pass_cmd_i: usize,
            image: (&'a gfx::Image, &'a gfx::ImageView),
        }

        struct BackDropInfo {
            image_view: gfx::ImageView,
            image: gfx::Image,
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
            use super::LayerContents::*;
            use super::ImageWrapMode::*;

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
                    let size = image
                        .image_data()
                        .get_presenter_ref(c.frame)
                        .unwrap()
                        .size();
                    let size_f = size.cast::<f32>();
                    let uv_matrix =
                        Matrix4::from_nonuniform_scale(1.0 / size_f.x, 1.0 / size_f.y, 1.0)
                            * model_mat_for_bounds(source);

                    c.compositor
                        .image_manager
                        .use_image(image, &mut c.image_ref_table);

                    Some((
                        (ImageContents::ManagedImage(image.clone()), sampler),
                        uv_matrix,
                        composite::SpriteFlagsBit::StraightAlpha.into(),
                        Vector4::new(1.0, 1.0, 1.0, opacity),
                    ))
                }
                &Solid(rgba) => Some((
                    (
                        (
                            c.compositor.white_image_view.clone(),
                            c.compositor.white_image.clone(),
                        ).into(),
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
                        (
                            (backdrop.image_view, backdrop.image).into(),
                            c.compositor.sampler_clamp.clone(),
                        ),
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
                        (
                            c.compositor.white_image_view.clone(),
                            c.compositor.white_image.clone(),
                        ).into(),
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
                let mut pixel_size = (size * cc.pixel_ratio).cast::<u32>();
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
                            *pass_i = *pass_i & !RENDER_PASS_BIT_USAGE_MASK
                                | RENDER_PASS_BIT_USAGE_SHADER_READ;
                        }
                        _ => unreachable!(),
                    }
                    c.cmds[rc.cmd_group_i].push(Cmd::EndPass);
                    cmd_group_i = rc.cmd_group_i;

                    backdrop = Some(BackDropInfo {
                        image_view: rc.image.1.clone(),
                        image: rc.image.0.clone(),
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
                let image = c.compositor
                    .device
                    .build_image()
                    .extents(&pixel_size[..])
                    .format(gfx::ImageFormat::SrgbRgba8)
                    .usage(flags![gfx::ImageUsage::{Render | Sampled}])
                    .build()?;

                c.compositor
                    .temp_res_pool
                    .add_image(&mut c.temp_res_table, image.clone());
                c.compositor.temp_res_pool.bind(
                    &mut c.temp_res_table,
                    c.compositor.backing_store_memory_type,
                    &image,
                )?;
                let image_view = c.compositor
                    .device
                    .new_image_view(&image, gfx::ImageLayout::ShaderRead)?;
                c.compositor
                    .temp_res_pool
                    .add_image_view(&mut c.temp_res_table, image_view.clone());

                c.rts.push(RenderTarget {
                    image: image.clone(),
                    extents: pixel_size,
                });

                let rt_i = c.rts.len() - 1;

                c.cmds[cmd_group_i].push(Cmd::BeginPass {
                    pass_i: RENDER_PASS_BIT_CLEAR | RENDER_PASS_BIT_USAGE_SHADER_READ,
                    rt_i,
                });

                // Render the contents and children
                {
                    let mut new_rc = RasterContext {
                        cmd_group_i,
                        begin_pass_cmd_i: c.cmds[cmd_group_i].len() - 1,
                        image: (&image, &image_view),
                    };
                    render_inner(cc, c, &mut new_rc, layer, inner_matrix, 1.0, backdrop)?;
                }

                c.cmds[cmd_group_i].push(Cmd::EndPass);

                if use_backdrop {
                    let (pass_i, rt_i) = saved.unwrap();
                    // Restart the interrupted render pass
                    c.cmds[rc.cmd_group_i].push(Cmd::BeginPass {
                        pass_i: pass_i & RENDER_PASS_BIT_USAGE_MASK,
                        rt_i,
                    });
                    rc.begin_pass_cmd_i = c.cmds[rc.cmd_group_i].len() - 1;
                }

                // Render the mask image
                let mask_contents = if let &Some(ref mask) = mask {
                    // Create a mask image
                    let mask_image = c.compositor
                        .device
                        .build_image()
                        .extents(&pixel_size[..])
                        .format(gfx::ImageFormat::SrgbBgra8)
                        .usage(flags![gfx::ImageUsage::{Render | Sampled}])
                        .build()?;

                    c.compositor
                        .temp_res_pool
                        .add_image(&mut c.temp_res_table, mask_image.clone());
                    c.compositor.temp_res_pool.bind(
                        &mut c.temp_res_table,
                        c.compositor.backing_store_memory_type,
                        &mask_image,
                    )?;
                    let mask_image_view = c.compositor
                        .device
                        .new_image_view(&mask_image, gfx::ImageLayout::ShaderRead)?;
                    c.compositor
                        .temp_res_pool
                        .add_image_view(&mut c.temp_res_table, mask_image_view.clone());

                    c.rts.push(RenderTarget {
                        image: mask_image.clone(),
                        extents: pixel_size,
                    });

                    let mask_rt_i = c.rts.len() - 1;

                    c.cmds.push(vec![
                        Cmd::BeginPass {
                            pass_i: RENDER_PASS_BIT_CLEAR | RENDER_PASS_BIT_USAGE_SHADER_READ,
                            rt_i: mask_rt_i,
                        },
                    ]);
                    let mask_cmd_group_i = c.cmds.len() - 1;

                    {
                        let mut mask_rc = RasterContext {
                            cmd_group_i: mask_cmd_group_i,
                            begin_pass_cmd_i: 0,
                            image: (&mask_image, &mask_image_view),
                        };

                        mask.for_each_node_of_r(|layer: &Layer| {
                            traverse(cc, c, &mut mask_rc, layer, inner_matrix, 1.0)
                        })?;
                    }

                    c.cmds[mask_cmd_group_i].push(Cmd::EndPass);

                    (
                        (mask_image_view, mask_image).into(),
                        c.compositor.sampler_clamp.clone(),
                    )
                } else {
                    (
                        (
                            c.compositor.white_image_view.clone(),
                            c.compositor.white_image.clone(),
                        ).into(),
                        c.compositor.sampler_clamp.clone(),
                    )
                };

                // Now composite the flattened contents to the parent raster
                // context's image
                let instance_i = c.sprites.len();
                let contents_i = c.contents.len();
                c.contents.push([
                    (
                        (image_view, image).into(),
                        c.compositor.sampler_clamp.clone(),
                    ),
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
        c.cmds.push(vec![
            Cmd::BeginPass {
                pass_i: RENDER_PASS_BIT_CLEAR | RENDER_PASS_BIT_USAGE_PRESENT,
                rt_i: 0,
            },
        ]);
        c.rts.push(RenderTarget {
            image: drawable.image().clone(),
            extents: Vector2::from(surface_props.extents),
        });
        if let &Some(ref root) = root {
            let drawable_image_view = c.compositor
                .device
                .new_image_view(drawable.image(), gfx::ImageLayout::ShaderRead)?;
            c.compositor
                .temp_res_pool
                .add_image_view(&mut c.temp_res_table, drawable_image_view.clone());

            let root_matrix = Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.5))
                * Matrix4::from_nonuniform_scale(2.0 / dpi_width, 2.0 / dpi_height, 0.0);

            let mut rc = RasterContext {
                cmd_group_i: 0,
                begin_pass_cmd_i: 0,
                image: (drawable.image(), &drawable_image_view),
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
            framebuffer: [Option<gfx::RenderTargetTable>; 6],
            rt: RenderTarget,
        }

        let ref mut compositor = c.compositor;

        let mut rt_data: Vec<_> = c.rts
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
        compositor
            .temp_res_pool
            .add_buffer(&mut c.temp_res_table, sprites_buf.clone());
        let sprites_alloc = compositor.temp_res_pool.bind(
            &mut c.temp_res_table,
            compositor.buffer_memory_type,
            &sprites_buf,
        )?;
        {
            use std::slice::from_raw_parts_mut;
            let sprites_slice = unsafe {
                from_raw_parts_mut(
                    compositor.temp_res_pool.as_ptr(&sprites_alloc)? as *mut composite::Sprite,
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
        let contents_images: Vec<[(gfx::ImageView, gfx::Image); 2]> = c.contents
            .iter()
            .map(|contents| {
                [
                    match contents[0].0 {
                        ImageContents::Image(ref image_view, ref image) => {
                            (image_view.clone(), image.clone())
                        }
                        ImageContents::ManagedImage(ref image_ref) => {
                            let resident_image = compositor.image_manager.get(&image_ref).unwrap();
                            (
                                resident_image.image_view().clone(),
                                resident_image.image().clone(),
                            )
                        }
                        ImageContents::Port(ref port) => {
                            let port_output = port_frame.get_output(port).unwrap();
                            (port_output.image_view.clone(), port_output.image.clone())
                        }
                    },
                    match contents[1].0 {
                        ImageContents::Image(ref image_view, ref image) => {
                            (image_view.clone(), image.clone())
                        }
                        _ => unreachable!(),
                    },
                ]
            })
            .collect();

        // Make argument tables
        let mut arg_pool;
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
                &at_global,
                &[
                    (
                        composite::ARG_G_SPRITE_PARAMS,
                        0,
                        [(0..sprites_size, &sprites_buf)][..].into(),
                    ),
                ],
            )?;

            at_contents = arg_pool
                .new_tables(
                    c.contents.len(),
                    &shaders.composite_arg_table_sigs[composite::ARG_TABLE_CONTENTS],
                )?
                .unwrap();

            let mut at_contents_image_views = Vec::with_capacity(c.contents.len() * 2);
            let mut at_contents_samplers = Vec::with_capacity(c.contents.len() * 2);

            for (contents, images) in c.contents.iter().zip(contents_images.iter()) {
                for (&(_, ref sampler), &(ref image_view, _)) in
                    contents[0..2].iter().zip(images.iter())
                {
                    at_contents_image_views.push(image_view);
                    at_contents_samplers.push(sampler);
                }
            }

            let mut at_contents_update_sets = Vec::with_capacity(c.contents.len() * 4);

            for i in 0..c.contents.len() {
                at_contents_update_sets.push((
                    composite::ARG_C_IMAGE,
                    0,
                    (&at_contents_image_views[i * 2..][..1]).into(),
                ));
                at_contents_update_sets.push((
                    composite::ARG_C_MASK,
                    0,
                    (&at_contents_image_views[i * 2 + 1..][..1]).into(),
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
                .map(|(update_sets, arg_table)| (arg_table, update_sets))
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
        let simple_barrier = compositor.device.build_barrier().build()?;

        // Encode the command buffer
        let mut cb = compositor.cmd_pool.begin_cmd_buffer()?;
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
                            builder.target(0, &rt_data.rt.image);
                            *fb = Some(builder.build()?);
                        }
                    }

                    let fb = rt_data[rt_i].framebuffer[pass_i].as_ref().unwrap();
                    enc = cb.encode_render(fb);

                    if let Some(ref fence) = fence.take() {
                        enc.wait_fence(fence, flags![gfx::Stage::{Copy}], &simple_barrier);
                    }

                    enc.bind_pipeline(&compositor.statesets[0].composite_pipeline);
                    enc.bind_vertex_buffers(0, &[(&compositor.box_vertices, 0)]);
                    enc.set_viewports(0, &[rt_data[rt_i].viewport]);
                    enc.bind_arg_table(composite::ARG_TABLE_GLOBAL, &[&at_global]);

                    enc.use_resource(gfx::ResourceUsage::Read, &[(&sprites_buf).into()]);
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
                                    let (src_stage, src_access) = port_output.fence_src;

                                    let barrier = compositor
                                        .device
                                        .build_barrier()
                                        .image(
                                            src_access,
                                            flags![gfx::AccessType::{FragmentRead}],
                                            &port_output.image,
                                            port_output.image_layout,
                                            gfx::ImageLayout::ShaderRead,
                                            &Default::default(),
                                        )
                                        .build()?;

                                    enc.wait_fence(&port_output.fence, src_stage, &barrier);
                                }
                                _ => {}
                            }
                            enc.use_resource(
                                gfx::ResourceUsage::Sample,
                                &[
                                    (&contents_images[contents_i][0].1).into(),
                                    (&contents_images[contents_i][1].1).into(),
                                ],
                            );
                            enc.bind_arg_table(
                                composite::ARG_TABLE_CONTENTS,
                                &[&at_contents[contents_i]],
                            );
                            let instance_i = instance_i as u32;
                            let count = count as u32;
                            enc.draw(0..4, instance_i..instance_i + count);
                        }
                    }
                }
            }
        }

        drawable.encode_prepare_present(&mut *cb, compositor.gfx_objects.main_queue.queue_family);

        cb.commit()?;
        compositor.main_queue.flush();

        self.frames.push_back(CompositeFrame {
            temp_res_table: c.temp_res_table,
            image_ref_table: c.image_ref_table,
            arg_pool,
            cb_state_tracker,
        });

        Ok(())
    }
}

impl Stateset {
    fn new(
        device: &gfx::Device,
        shaders: &CompositorShaders,
        framebuffer_format: gfx::ImageFormat,
    ) -> Result<Self> {
        let render_passes: Vec<_> = (0..6)
            .map(|i| {
                let usage = i & RENDER_PASS_BIT_USAGE_MASK;

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
                    .set_store_op(gfx::StoreOp::Store)
                    .set_initial_layout(if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                        gfx::ImageLayout::Undefined
                    } else {
                        gfx::ImageLayout::General
                    })
                    .set_final_layout(match usage {
                        RENDER_PASS_BIT_USAGE_PRESENT => gfx::ImageLayout::Present,
                        RENDER_PASS_BIT_USAGE_SHADER_READ => gfx::ImageLayout::ShaderRead,
                        RENDER_PASS_BIT_USAGE_GENERAL => gfx::ImageLayout::General,
                        _ => unreachable!(),
                    });

                builder.subpass_color_targets(&[Some((0, gfx::ImageLayout::RenderWrite))]);
                builder.end();

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
