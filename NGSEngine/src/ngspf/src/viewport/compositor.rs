//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, Mutex};

use atomic_refcell::AtomicRefCell;
use cgmath::{Vector3, Matrix4};
use cgmath::*;
use gfx;
use gfx::core::Backend;
use gfx::prelude::*;

use prelude::*;
use gfxutils::DeviceUtils;
use context::{NodeRef, PresenterFrame};
use super::{WorkspaceDevice, Layer};
use super::temprespool::TempResPool;
use super::uploader::Uploader;

/// Compositor.
///
/// # Notes Regarding Memory Management
///
/// `Compositor` does not free device allocations on drop.
#[derive(Debug)]
pub struct Compositor<B: Backend> {
    device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
    statesets: Vec<Stateset<B>>,
    shaders: CompositorShaders<B>,

    box_vertices: B::Buffer,

    white_image: B::Image,
    white_image_view: B::ImageView,

    sampler_repeat: B::Sampler,
    sampler_clamp: B::Sampler,
}

#[derive(Debug)]
struct CompositorShaders<B: Backend> {
    composite_ds_layouts: [B::DescriptorSetLayout; 2],
    composite_layout: B::PipelineLayout,
    composite_module_frag: B::ShaderModule,
    composite_module_vert: B::ShaderModule,
}

static BOX_VERTICES: &[[u16; 2]] = &[[0, 0], [1, 0], [0, 1], [1, 1]];

mod composite {
    use include_data;
    use gfx::core::*;
    use cgmath::{Matrix4, Vector4};
    use ngsenumflags::BitFlags;

    pub static SPIRV_FRAG: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/composite.frag.spv"));
    pub static SPIRV_VERT: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/composite.vert.spv"));

    // Vertex attribute locations
    pub static VA_POSITION: VertexAttributeLocation = 0;

    // Descriptor set binding locations
    pub static DSB_GLOBAL: DescriptorSetBindingLocation = 0;
    pub static DSB_CONTENTS: DescriptorSetBindingLocation = 1;

    // Descriptor binding locations
    pub static DB_G_SPRITE_PARAMS: DescriptorBindingLocation = 0;

    pub static DB_C_IMAGE: DescriptorBindingLocation = 0;
    pub static DB_C_MASK: DescriptorBindingLocation = 1;

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
struct Stateset<B: Backend> {
    framebuffer_format: gfx::core::ImageFormat,
    render_passes: Vec<B::RenderPass>,

    composite_pipeline: B::GraphicsPipeline,
}

const RENDER_PASS_BIT_CLEAR: usize = 1 << 0;
const RENDER_PASS_BIT_USAGE_MASK: usize = 0b11 << 1;
const RENDER_PASS_BIT_USAGE_PRESENT: usize = 0b00 << 1;
const RENDER_PASS_BIT_USAGE_SHADER_READ: usize = 0b01 << 1;
const RENDER_PASS_BIT_USAGE_GENERAL: usize = 0b10 << 1;

#[derive(Debug)]
pub struct CompositorWindow<B: Backend> {
    compositor: Arc<Compositor<B>>,
    command_buffers: Vec<Arc<AtomicRefCell<B::CommandBuffer>>>,
    command_buffer_index: usize,
    temp_res_pool: TempResPool<B>,
}

#[derive(Debug)]
pub struct CompositeContext<'a, B: Backend> {
    pub workspace_device: &'a WorkspaceDevice<B>,
    pub schedule_next_frame: bool,
    /// Command buffers to be submitted to the device (after calls to `composite` are done).
    pub command_buffers: Vec<Arc<AtomicRefCell<B::CommandBuffer>>>,
    pub pixel_ratio: f32,
    pub uploader: &'a Uploader<B>,
}

impl<B: Backend> Compositor<B> {
    pub fn new(ws_device: &WorkspaceDevice<B>) -> gfx::core::Result<Self> {
        use gfx::core::*;

        let ref device = ws_device.objects().gfx_device();
        let factory = device.factory();

        let utils = DeviceUtils::<B>::new(device);

        let composite_ds_layouts = [
            factory.make_descriptor_set_layout(
                &DescriptorSetLayoutDescription {
                    bindings: &[
                        DescriptorSetLayoutBinding {
                            location: composite::DB_G_SPRITE_PARAMS,
                            descriptor_type: DescriptorType::StorageBuffer,
                            num_elements: 1,
                            stage_flags: ShaderStage::Vertex.into(),
                            immutable_samplers: None,
                        },
                    ],
                },
            )?,
            factory.make_descriptor_set_layout(
                &DescriptorSetLayoutDescription {
                    bindings: &[
                        DescriptorSetLayoutBinding {
                            location: composite::DB_C_IMAGE,
                            descriptor_type: DescriptorType::CombinedImageSampler,
                            num_elements: 1,
                            stage_flags: ShaderStage::Fragment.into(),
                            immutable_samplers: None,
                        },
                        DescriptorSetLayoutBinding {
                            location: composite::DB_C_MASK,
                            descriptor_type: DescriptorType::CombinedImageSampler,
                            num_elements: 1,
                            stage_flags: ShaderStage::Fragment.into(),
                            immutable_samplers: None,
                        },
                    ],
                },
            )?,
        ];
        let composite_layout = factory.make_pipeline_layout(&PipelineLayoutDescription {
            descriptor_set_layouts: &[&composite_ds_layouts[0], &composite_ds_layouts[1]],
        })?;
        let composite_module_vert = factory.make_shader_module(&ShaderModuleDescription {
            spirv_code: composite::SPIRV_VERT.as_u32_slice(),
        })?;
        let composite_module_frag = factory.make_shader_module(&ShaderModuleDescription {
            spirv_code: composite::SPIRV_FRAG.as_u32_slice(),
        })?;

        let shaders = CompositorShaders {
            // "composite" shader
            composite_ds_layouts,
            composite_layout,
            composite_module_vert,
            composite_module_frag,
        };

        let white_image = utils.make_preinitialized_image_no_mip(
            &mut ws_device.objects().heap().lock().unwrap(),
            &[0xffffffffu32],
            ImageDescription {
                usage: ImageUsage::Sampled.into(),
                format: ImageFormat::Rgba8(Signedness::Unsigned, Normalizedness::Normalized),
                extent: Vector3::new(1, 1, 1),
                ..Default::default()
            },
            PipelineStage::FragmentShader.into(),
            AccessType::ShaderRead.into(),
            ImageLayout::ShaderRead,
            DeviceEngine::Universal,
        )?;

        let sampler_repeat = factory.make_sampler(&SamplerDescription {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: MipmapMode::Linear,
            address_mode: [SamplerAddressMode::Repeat; 3],
            lod_min_clamp: 0f32,
            lod_max_clamp: 0.5f32,
            max_anisotropy: 1,
            compare_function: None,
            border_color: SamplerBorderColor::FloatOpaqueBlack,
            unnormalized_coordinates: false,
        })?;
        let sampler_clamp = factory.make_sampler(&SamplerDescription {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: MipmapMode::Linear,
            address_mode: [SamplerAddressMode::Repeat; 3],
            lod_min_clamp: 0f32,
            lod_max_clamp: 0.5f32,
            max_anisotropy: 1,
            compare_function: None,
            border_color: SamplerBorderColor::FloatOpaqueBlack,
            unnormalized_coordinates: false,
        })?;

        Ok(Self {
            statesets: vec![
                Stateset::new(
                    &**ws_device.objects().gfx_device(),
                    &shaders,
                    ImageFormat::SrgbBgra8
                )?,
            ],
            shaders,

            box_vertices: utils
                .make_preinitialized_buffer(
                    &mut ws_device.objects().heap().lock().unwrap(),
                    &BOX_VERTICES,
                    BufferUsage::VertexBuffer.into(),
                    PipelineStage::VertexInput.into(),
                    AccessType::VertexAttributeRead.into(),
                    DeviceEngine::Universal,
                )?
                .0,

            white_image: white_image.0,
            white_image_view: white_image.1,

            sampler_repeat,
            sampler_clamp,

            heap: Arc::clone(ws_device.objects().heap()),
            device: Arc::clone(ws_device.objects().gfx_device()),
        })
    }
}

impl<B: Backend> CompositorWindow<B> {
    pub fn new(compositor: Arc<Compositor<B>>) -> gfx::core::Result<Self> {
        let device = Arc::clone(&compositor.device);

        Ok(Self {
            command_buffers: (0..2)
                .map(|_| {
                    let cb = device.main_queue().make_command_buffer()?;
                    cb.set_label(Some("compositor main command buffer"));
                    Ok(Arc::new(AtomicRefCell::new(cb)))
                })
                .collect::<Result<_, _>>()?,
            command_buffer_index: 0,

            temp_res_pool: TempResPool::new(
                Arc::clone(&compositor.device),
                Arc::clone(&compositor.heap),
            )?,

            compositor,
        })
    }

    pub fn frame_description(&self) -> gfx::wsi::FrameDescription {
        gfx::wsi::FrameDescription {
            acquiring_engines: gfx::core::DeviceEngine::Universal.into(),
            releasing_engines: gfx::core::DeviceEngine::Universal.into(),
        }
    }

    pub fn composite<D>(
        &mut self,
        context: &mut CompositeContext<B>,
        root: &Option<NodeRef>,
        frame: &PresenterFrame,
        drawable: &D,
        drawable_info: &gfx::wsi::DrawableInfo,
    ) -> gfx::core::Result<()>
    where
        D: gfx::wsi::Drawable<Backend = B>,
    {
        let device: &B::Device = context.workspace_device.objects().gfx_device();
        let image_view = device.factory().make_image_view(
            &gfx::core::ImageViewDescription {
                image_type: gfx::core::ImageType::TwoD,
                image: drawable.image(),
                format: drawable_info.format,
                range: gfx::core::ImageSubresourceRange::default(),
            },
        )?;

        use std::mem::size_of;
        use std::ptr::copy;

        use ngsbase::Box2;
        use ngsbase::prelude::*;
        use gfx::core::*;

        enum Cmd {
            BeginPass { pass_i: usize, rt_i: usize },
            EndPass,
            EndPassForPresentation,
            Sprite {
                instance_i: usize,
                contents_i: usize,
                count: usize,
            },
        }

        struct LocalContext<'a, B: Backend> {
            frame: &'a PresenterFrame,
            device: &'a B::Device,
            sprites: Vec<composite::Sprite>,
            contents: Vec<[(B::ImageView, B::Sampler); 2]>,
            cmds: Vec<Vec<Cmd>>,
            rts: Vec<RenderTarget<B>>,
        }

        struct RenderTarget<B: Backend> {
            image_view: B::ImageView,
            extents: Vector2<u32>,
        }

        struct RasterContext<'a, B: Backend> {
            cmd_group_i: usize,
            begin_pass_cmd_i: usize,
            image: (&'a B::Image, &'a B::ImageView),
        }

        struct BackDropInfo<B: Backend> {
            image_view: B::ImageView,
            uv_matrix: Matrix4<f32>,
        }

        fn model_mat_for_bounds(bounds: &Box2<f32>) -> Matrix4<f32> {
            let size = bounds.size();
            Matrix4::from_translation(bounds.min.to_vec().extend(0.0)) *
                Matrix4::from_nonuniform_scale(size.x, size.y, 1.0)
        }

        fn render_inner<B: Backend>(
            this: &mut CompositorWindow<B>,
            cc: &mut CompositeContext<B>,
            c: &mut LocalContext<B>,
            rc: &mut RasterContext<B>,
            layer: &Layer,
            matrix: Matrix4<f32>,
            opacity: f32,
            backdrop: Option<BackDropInfo<B>>,
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
                        Repeat => this.compositor.sampler_repeat.clone(),
                        Clamp => this.compositor.sampler_clamp.clone(),
                    };
                    let size = image
                        .image_data()
                        .get_presenter_ref(c.frame)
                        .unwrap()
                        .size();
                    let size_f = size.cast::<f32>();
                    let uv_matrix =
                        Matrix4::from_nonuniform_scale(1.0 / size_f.x, 1.0 / size_f.y, 1.0) *
                            model_mat_for_bounds(source);

                    let ri = cc.uploader.get(image).unwrap();
                    // TODO: wait for fence
                    Some((
                        (ri.image_view().clone(), sampler),
                        uv_matrix,
                        composite::SpriteFlagsBit::StraightAlpha.into(),
                    ))
                }
                &Port(_) => unimplemented!(),
                &BackDrop => {
                    let backdrop = backdrop.expect("BackDrop used without FlattenContents");
                    Some((
                        (backdrop.image_view, this.compositor.sampler_clamp.clone()),
                        backdrop.uv_matrix,
                        composite::SpriteFlags::empty(),
                    ))
                }
            };

            if let Some((image_view, uv_matrix, flags)) = sprite_info {
                let instance_i = c.sprites.len();
                let contents_i = c.contents.len();
                c.contents.push(
                    [
                        image_view,
                        (
                            this.compositor.white_image_view.clone(),
                            this.compositor.sampler_clamp.clone(),
                        ),
                    ],
                );
                c.sprites.push(composite::Sprite {
                    matrix: model_matrix,
                    uv_matrix,
                    color: Vector4::new(1.0, 1.0, 1.0, opacity),
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
                    traverse(this, cc, c, rc, layer, matrix, opacity)
                })?;
            }

            Ok(())
        }

        fn traverse<B: Backend>(
            this: &mut CompositorWindow<B>,
            cc: &mut CompositeContext<B>,
            c: &mut LocalContext<B>,
            rc: &mut RasterContext<B>,
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
                let inner_matrix = Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.5)) *
                    Matrix4::from_nonuniform_scale(2.0 / size.x, 2.0 / size.y, 0.0) *
                    Matrix4::from_translation(-bounds.min.to_vec().extend(0.0));

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
                            *pass_i = *pass_i & !RENDER_PASS_BIT_USAGE_MASK |
                                RENDER_PASS_BIT_USAGE_SHADER_READ;
                        }
                        _ => unreachable!(),
                    }
                    c.cmds[rc.cmd_group_i].push(Cmd::EndPass);
                    cmd_group_i = rc.cmd_group_i;

                    backdrop = Some(BackDropInfo {
                        image_view: rc.image.1.clone(),
                        uv_matrix: Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.0)) *
                            Matrix4::from_nonuniform_scale(0.5, 0.5, 1.0) *
                            model_matrix,
                    });
                } else {
                    // Create a new CB that are scheduled before the parent
                    // raster context's CB.
                    c.cmds.push(Vec::new());
                    cmd_group_i = c.cmds.len() - 1;
                    backdrop = None;
                }

                // Create a backing store image
                let temp_image = this.temp_res_pool.allocate_image(&ImageDescription {
                    usage: ImageUsage::ColorAttachment | ImageUsage::Sampled,
                    format: ImageFormat::SrgbRgba8,
                    extent: pixel_size.extend(1),
                    ..Default::default()
                })?;
                {
                    // FIXME: insert appropriate barrier for resource acqusition
                    let mut ops = this.temp_res_pool.ops_image(&temp_image);
                    *ops.image_layout_mut().unwrap() = ImageLayout::ShaderRead;
                    *ops.stage_access_type_mut().0 = PipelineStage::FragmentShader.into();
                    *ops.stage_access_type_mut().1 = AccessType::ShaderRead.into();
                }
                let image = temp_image.image();
                let image_view = c.device.factory().make_image_view(&ImageViewDescription {
                    image_type: ImageType::TwoD,
                    image,
                    format: ImageFormat::SrgbRgba8,
                    range: ImageSubresourceRange::default(),
                })?;
                c.rts.push(RenderTarget {
                    image_view: image_view.clone(),
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
                        image: (image, &image_view),
                    };
                    render_inner(this, cc, c, &mut new_rc, layer, inner_matrix, 1.0, backdrop)?;
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
                    let mask_temp_image = this.temp_res_pool.allocate_image(&ImageDescription {
                        usage: ImageUsage::ColorAttachment | ImageUsage::Sampled,
                        format: ImageFormat::SrgbBgra8,
                        extent: pixel_size.extend(1),
                        ..Default::default()
                    })?;
                    {
                        // FIXME: insert appropriate barrier for resource acqusition
                        let mut ops = this.temp_res_pool.ops_image(&mask_temp_image);
                        *ops.image_layout_mut().unwrap() = ImageLayout::ShaderRead;
                        *ops.stage_access_type_mut().0 = PipelineStage::FragmentShader.into();
                        *ops.stage_access_type_mut().1 = AccessType::ShaderRead.into();
                    }
                    let mask_image = mask_temp_image.image();
                    let mask_image_view =
                        c.device.factory().make_image_view(&ImageViewDescription {
                            image_type: ImageType::TwoD,
                            image: mask_image,
                            format: ImageFormat::SrgbBgra8,
                            range: ImageSubresourceRange::default(),
                        })?;

                    c.rts.push(RenderTarget {
                        image_view: mask_image_view.clone(),
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
                            image: (mask_image, &mask_image_view),
                        };

                        mask.for_each_node_of_r(|layer: &Layer| {
                            traverse(this, cc, c, &mut mask_rc, layer, inner_matrix, 1.0)
                        })?;
                    }

                    c.cmds[mask_cmd_group_i].push(Cmd::EndPass);

                    (mask_image_view, this.compositor.sampler_clamp.clone())
                } else {
                    (
                        this.compositor.white_image_view.clone(),
                        this.compositor.sampler_clamp.clone(),
                    )
                };

                // Now composite the flattened contents to the parent raster
                // context's image
                let instance_i = c.sprites.len();
                let contents_i = c.contents.len();
                c.contents.push(
                    [
                        (image_view, this.compositor.sampler_clamp.clone()),
                        mask_contents,
                    ],
                );
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
                render_inner(this, cc, c, rc, layer, local_matrix, opacity, None)?;
            }

            Ok(())
        }

        let dpi_width = drawable_info.extents.x as f32 / context.pixel_ratio;
        let dpi_height = drawable_info.extents.y as f32 / context.pixel_ratio;

        let mut c = LocalContext {
            frame,
            device,
            sprites: Vec::new(),
            contents: Vec::new(),
            cmds: vec![
                vec![
                    Cmd::BeginPass {
                        pass_i: RENDER_PASS_BIT_CLEAR | RENDER_PASS_BIT_USAGE_PRESENT,
                        rt_i: 0,
                    },
                ],
            ],
            rts: vec![
                RenderTarget {
                    image_view: image_view.clone(),
                    extents: drawable_info.extents.truncate(),
                },
            ],
        };
        if let &Some(ref root) = root {
            let root_matrix = Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.5)) *
                Matrix4::from_nonuniform_scale(2.0 / dpi_width, 2.0 / dpi_height, 0.0);
            let mut rc = RasterContext {
                cmd_group_i: 0,
                begin_pass_cmd_i: 0,
                image: (drawable.image(), &image_view),
            };
            root.for_each_node_of_r(|layer: &Layer| {
                traverse(self, context, &mut c, &mut rc, layer, root_matrix, 1.0)
            })?;
        }
        c.cmds[0].push(Cmd::EndPassForPresentation);

        // Collect various data
        struct RtData<B: Backend> {
            viewport: Viewport,
            framebuffer: [Option<B::Framebuffer>; 6],
            rt: RenderTarget<B>,
        }

        let ref compositor = self.compositor;
        let ref shaders = compositor.shaders;

        let mut rt_data: Vec<_> = c.rts
            .into_iter()
            .map(|rt| {
                RtData {
                    viewport: Viewport {
                        x: 0f32,
                        y: 0f32,
                        width: rt.extents.x as f32,
                        height: rt.extents.y as f32,
                        min_depth: 0f32,
                        max_depth: 1f32,
                    },
                    framebuffer: Default::default(),
                    rt,
                }
            })
            .collect();

        // Prepare to upload `Sprite`
        let sprites_size = (size_of::<composite::Sprite>() * c.sprites.len()) as DeviceSize;
        let sprites_buf = self.temp_res_pool
            .allocate_buffer(&BufferDescription {
                usage: BufferUsage::TransferDestination | BufferUsage::StorageBuffer,
                size: sprites_size,
                storage_mode: StorageMode::Private,
            })?
            .into_buffer();
        let sprites_st_buf = self.temp_res_pool.allocate_buffer(&BufferDescription {
            usage: BufferUsage::TransferSource.into(),
            size: sprites_size,
            storage_mode: StorageMode::Shared,
        })?;
        // TODO: check access type etc.
        {
            let mut heap = self.compositor.heap.lock().unwrap();
            let mut ops = self.temp_res_pool.ops_buffer(&sprites_st_buf);
            let mut map = heap.map_memory(ops.allocation_mut())?;
            unsafe {
                copy(
                    c.sprites.as_ptr(),
                    map.as_mut_ptr() as *mut composite::Sprite,
                    c.sprites.len(),
                );
            }
        }
        let sprites_st_buf = sprites_st_buf.into_buffer();

        // Make descriptor sets
        let mut ds_pool = device.factory().make_descriptor_pool(
            &DescriptorPoolDescription {
                max_num_sets: c.contents.len() + 1,
                supports_deallocation: false,
                pool_sizes: &[
                    DescriptorPoolSize {
                        descriptor_type: DescriptorType::StorageBuffer,
                        num_descriptors: 1,
                    },
                    DescriptorPoolSize {
                        descriptor_type: DescriptorType::CombinedImageSampler,
                        num_descriptors: c.contents.len() * 2,
                    },
                ],
            },
        )?;
        let ds_global = ds_pool
            .make_descriptor_set(&DescriptorSetDescription {
                layout: &shaders.composite_ds_layouts[composite::DSB_GLOBAL],
            })?
            .unwrap()
            .0;
        ds_global.update(
            &[
                WriteDescriptorSet {
                    start_binding: composite::DB_G_SPRITE_PARAMS,
                    start_index: 0,
                    elements: WriteDescriptors::StorageBuffer(
                        &[
                            DescriptorBuffer {
                                buffer: &sprites_buf,
                                offset: 0,
                                range: sprites_size,
                            },
                        ],
                    ),
                },
            ],
        );

        let ds_contents: Vec<_> = c.contents
            .iter()
            .map(|c| {
                let ds = ds_pool
                    .make_descriptor_set(&DescriptorSetDescription {
                        layout: &shaders.composite_ds_layouts[composite::DSB_CONTENTS],
                    })?
                    .unwrap()
                    .0;

                ds.update(
                    &[
                        WriteDescriptorSet {
                            start_binding: composite::DB_C_IMAGE,
                            start_index: 0,
                            elements: WriteDescriptors::CombinedImageSampler(
                                &[
                                    (
                                        DescriptorImage {
                                            image_view: &c[0].0,
                                            image_layout: ImageLayout::ShaderRead,
                                        },
                                        &c[0].1,
                                    ),
                                    (
                                        DescriptorImage {
                                            image_view: &c[1].0,
                                            image_layout: ImageLayout::ShaderRead,
                                        },
                                        &c[1].1,
                                    ),
                                ],
                            ),
                        },
                    ],
                );

                Ok(ds)
            })
            .collect::<Result<_>>()?;

        self.command_buffer_index = (self.command_buffer_index + 1) % self.command_buffers.len();
        let cb_cell = Arc::clone(&self.command_buffers[self.command_buffer_index]);
        let mut cb_cell_2 = Some(Arc::clone(&cb_cell));

        cb_cell.borrow().wait_completion()?;
        self.temp_res_pool.retire_old_frames();

        {
            let mut cb = cb_cell.borrow_mut();
            cb.begin_encoding();

            cb.begin_copy_pass(DeviceEngine::Universal);
            {
                cb.begin_debug_group(&DebugMarker::new("stage sprites"));
                cb.acquire_resource(
                    PipelineStage::Transfer.into(),
                    AccessType::TransferRead.into(),
                    DeviceEngine::Host,
                    &SubresourceWithLayout::Buffer {
                        buffer: &sprites_st_buf,
                        offset: 0,
                        len: sprites_size,
                    },
                );
                cb.copy_buffer(&sprites_st_buf, 0, &sprites_buf, 0, sprites_size);
                cb.resource_barrier(
                    PipelineStage::Transfer.into(),
                    AccessType::TransferWrite.into(),
                    PipelineStage::VertexShader.into(),
                    AccessType::ShaderRead.into(),
                    &SubresourceWithLayout::Buffer {
                        buffer: &sprites_st_buf,
                        offset: 0,
                        len: sprites_size,
                    },
                );
                cb.end_debug_group();
            }
            cb.end_pass();

            for cmds in c.cmds.iter().rev() {
                for cmd in cmds.iter() {
                    match cmd {
                        &Cmd::BeginPass { pass_i, rt_i } => {
                            {
                                let ref mut rt_data = rt_data[rt_i];
                                let ref mut fb = rt_data.framebuffer[pass_i];
                                if fb.is_none() {
                                    *fb = Some(device.factory().make_framebuffer(
                                        &FramebufferDescription {
                                            render_pass: &compositor.statesets[0].render_passes
                                                [pass_i],
                                            attachments: &[
                                                FramebufferAttachmentDescription {
                                                    image_view: &rt_data.rt.image_view,
                                                    clear_values: ClearValues::ColorFloat(
                                                        [0.0, 0.0, 0.0, 0.0],
                                                    ),
                                                },
                                            ],
                                            width: rt_data.rt.extents.x,
                                            height: rt_data.rt.extents.y,
                                            num_layers: 1,
                                        },
                                    )?);
                                }
                            }
                            let fb = rt_data[rt_i].framebuffer[pass_i].as_ref().unwrap();
                            cb.begin_render_pass(fb, DeviceEngine::Universal);
                            cb.begin_render_subpass(RenderPassContents::Inline);

                            if rt_i == 0 {
                                if let Some(fence) = drawable.acquiring_fence() {
                                    cb.wait_fence(
                                        PipelineStage::ColorAttachmentOutput.into(),
                                        AccessType::ColorAttachmentWrite.into(),
                                        fence,
                                    );
                                }
                            }

                            cb.bind_graphics_pipeline(
                                &self.compositor.statesets[0].composite_pipeline,
                            );
                            cb.set_viewport(&rt_data[rt_i].viewport);
                            cb.bind_graphics_descriptor_sets(
                                &shaders.composite_layout,
                                composite::DSB_GLOBAL,
                                &[&ds_global],
                                &[],
                            );
                            cb.bind_vertex_buffers(0, &[(&compositor.box_vertices, 0)]);
                        }
                        &Cmd::EndPass => {
                            cb.end_render_subpass();
                            cb.end_pass();
                        }
                        &Cmd::EndPassForPresentation => {
                            if let Some(fence) = drawable.releasing_fence() {
                                cb.update_fence(
                                    PipelineStage::ColorAttachmentOutput.into(),
                                    AccessType::ColorAttachmentWrite.into(),
                                    fence,
                                );
                            }
                            cb.end_render_subpass();
                            drawable.finalize(
                                &mut cb,
                                PipelineStage::ColorAttachmentOutput.into(),
                                AccessType::ColorAttachmentWrite.into(),
                                ImageLayout::Present,
                            );
                            self.temp_res_pool.finalize_frame(
                                cb_cell_2.take().unwrap(),
                                &mut cb,
                            );
                            cb.end_pass();
                        }
                        &Cmd::Sprite {
                            instance_i,
                            contents_i,
                            count,
                        } => {
                            cb.bind_graphics_descriptor_sets(
                                &shaders.composite_layout,
                                composite::DSB_CONTENTS,
                                &[&ds_contents[contents_i]],
                                &[],
                            );
                            let instance_i = instance_i as u32;
                            let count = count as u32;
                            cb.draw(0..4, instance_i..instance_i + count);
                        }
                    }
                }
            }
            cb.end_encoding()?;
        }

        context.command_buffers.push(cb_cell);

        Ok(())
    }
}

impl<B: Backend> Stateset<B> {
    fn new(
        device: &B::Device,
        shaders: &CompositorShaders<B>,
        framebuffer_format: gfx::core::ImageFormat,
    ) -> gfx::core::Result<Self> {
        use gfx::core::*;
        use gfx::core::Signedness::*;
        use gfx::core::Normalizedness::*;
        use gfx::core::VectorWidth::*;

        let spb = RenderSubpassDescription {
            input_attachments: &[],
            color_attachments: &[
                RenderPassAttachmentReference {
                    attachment_index: Some(0),
                    layout: ImageLayout::ColorAttachment,
                },
            ],
            depth_stencil_attachment: None,
            preserve_attachments: &[],
        };

        let render_passes: Vec<_> = (0..6)
            .map(|i| {
                let usage = i & RENDER_PASS_BIT_USAGE_MASK;

                let desc = RenderPassDescription {
                    attachments: &[
                        RenderPassAttachmentDescription {
                            may_alias: false,
                            format: framebuffer_format,
                            load_op: if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                                AttachmentLoadOp::Clear
                            } else {
                                AttachmentLoadOp::Load
                            },
                            store_op: AttachmentStoreOp::Store,
                            stencil_load_op: AttachmentLoadOp::DontCare,
                            stencil_store_op: AttachmentStoreOp::DontCare,
                            initial_layout: if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                                ImageLayout::Undefined
                            } else {
                                ImageLayout::General
                            },
                            final_layout: match usage {
                                RENDER_PASS_BIT_USAGE_PRESENT => ImageLayout::Present,
                                RENDER_PASS_BIT_USAGE_SHADER_READ => ImageLayout::ShaderRead,
                                RENDER_PASS_BIT_USAGE_GENERAL => ImageLayout::General,
                                _ => unreachable!(),
                            },
                        },
                    ],
                    subpasses: &[spb],
                    dependencies: &[],
                };

                let render_pass = device.factory().make_render_pass(&desc);
                if let Ok(ref render_pass) = render_pass {
                    render_pass.set_label(Some("Compositor render pass"));
                }
                render_pass
            })
            .collect::<Result<_>>()?;

        let color_attachments = &[Default::default()];
        let composite_pipeline = device.factory().make_graphics_pipeline(
            &GraphicsPipelineDescription {
                label: Some("Composite"),
                shader_stages: &[
                    ShaderStageDescription {
                        stage: ShaderStage::Fragment,
                        module: &shaders.composite_module_frag,
                        entry_point_name: "main",
                    },
                    ShaderStageDescription {
                        stage: ShaderStage::Vertex,
                        module: &shaders.composite_module_vert,
                        entry_point_name: "main",
                    },
                ],
                vertex_buffers: &[
                    VertexBufferLayoutDescription {
                        binding: 0,
                        stride: 4,
                        input_rate: VertexInputRate::Vertex,
                    },
                ],
                vertex_attributes: &[
                    VertexAttributeDescription {
                        location: composite::VA_POSITION,
                        binding: 0,
                        format: VertexFormat(Vector2, ScalarFormat::I16(Unsigned, Unnormalized)),
                        offset: 0,
                    },
                ],
                topology: PrimitiveTopology::TriangleStrip,
                rasterizer: Some(GraphicsPipelineRasterizerDescription {
                    viewport: StaticOrDynamic::Dynamic,
                    cull_mode: CullMode::None,
                    color_attachments,
                    depth_write: false,
                    depth_test: CompareFunction::Always,
                    ..Default::default()
                }),
                pipeline_layout: &shaders.composite_layout,
                render_pass: &render_passes[0],
                subpass_index: 0,
            },
        )?;

        Ok(Self {
            framebuffer_format,
            render_passes,
            composite_pipeline,
        })
    }
}
