//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use glium::{
    backend::Facade,
    implement_vertex,
    index::{NoIndices, PrimitiveType},
    program,
    texture::{self, RawImage2d, Texture2d},
    uniform,
    uniforms::Sampler,
    Program, Surface, VertexBuffer,
};
use stygian::DepthImage;

pub struct DepthVis {
    vb: VertexBuffer<Vertex>,
    program: Program,
}

#[derive(Debug, Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

implement_vertex!(Vertex, pos, uv);

impl DepthVis {
    pub fn new(facade: &impl Facade) -> Self {
        Self {
            vb: VertexBuffer::new(
                facade,
                &[
                    Vertex {
                        pos: [-1.0, -1.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        pos: [1.0, -1.0],
                        uv: [1.0, 0.0],
                    },
                    Vertex {
                        pos: [-1.0, 1.0],
                        uv: [0.0, 1.0],
                    },
                    Vertex {
                        pos: [1.0, 1.0],
                        uv: [1.0, 1.0],
                    },
                ],
            )
            .unwrap(),
            program: program!(facade,
            100 => {
                    vertex: r"
                        #version 100

                        attribute highp vec2 pos;
                        attribute highp vec2 uv;
                        varying highp vec2 v_uv;

                        void main() {
                            v_uv = uv;
                            gl_Position = vec4(pos * 0.2 + 0.7, 0.0, 1.0);
                        }
                    ",
                    fragment: r"
                        #version 100

                        varying highp vec2 v_uv;
                        uniform mediump sampler2D u_texture;

                        void main() {
                            mediump float x = texture2D(u_texture, v_uv).x;
                            x = sqrt(x);
                            gl_FragColor = vec4(x, x, x, 1.0);
                        }
                    ",
            })
            .unwrap(),
        }
    }

    pub fn draw(&mut self, facade: &impl Facade, target: &mut impl Surface, image: &DepthImage) {
        let image = RawImage2d {
            data: image.pixels().into(),
            width: image.size().x as u32,
            height: image.size().y as u32,
            format: texture::ClientFormat::F32,
        };
        let texture = Texture2d::with_format(
            facade,
            image,
            texture::UncompressedFloatFormat::F32,
            texture::MipmapsOption::NoMipmap,
        )
        .unwrap();
        let sampler =
            Sampler::new(&texture).magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest);

        let uniforms = uniform! {
            u_texture: sampler,
        };

        let params = glium::DrawParameters {
            ..Default::default()
        };

        target
            .draw(
                &self.vb,
                &NoIndices(PrimitiveType::TriangleStrip),
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
}
