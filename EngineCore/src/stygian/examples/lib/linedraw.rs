//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Point2;
use glium::{
    backend::Facade,
    implement_vertex,
    index::{NoIndices, PrimitiveType},
    program, uniform, Program, Surface, VertexBuffer,
};

pub struct LineDraw {
    vb: Vec<Vertex>,
    gpu_vb: VertexBuffer<Vertex>,
    program: Program,
}

#[derive(Debug, Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
    color: [u8; 4],
}

implement_vertex!(Vertex, pos, color);

const CAPACITY: usize = 4096;

impl LineDraw {
    pub fn new(facade: &impl Facade) -> Self {
        Self {
            vb: Vec::with_capacity(CAPACITY),
            gpu_vb: VertexBuffer::empty_dynamic(facade, CAPACITY).unwrap(),
            program: program!(facade,
            100 => {
                    vertex: r"
                        #version 100

                        attribute highp vec2 pos;
                        attribute highp vec4 color;
                        varying lowp vec4 v_color;

                        void main() {
                            v_color = color / 255.0;
                            gl_Position = vec4(pos, 0.0, 1.0);
                        }
                    ",
                    fragment: r"
                        #version 100

                        varying lowp vec4 v_color;

                        void main() {
                            gl_FragColor = v_color;
                        }
                    ",
            })
            .unwrap(),
        }
    }

    pub fn flush(&mut self, target: &mut impl Surface) {
        if self.vb.len() == 0 {
            return;
        }

        let uniforms = uniform! {};

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        self.gpu_vb
            .slice(0..self.vb.len())
            .unwrap()
            .write(&self.vb[..]);

        target
            .draw(
                self.gpu_vb.slice(0..self.vb.len()).unwrap(),
                &NoIndices(PrimitiveType::LinesList),
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();

        self.vb.clear();
    }

    pub fn push(&mut self, color: [u8; 4], vertices: impl IntoIterator<Item = Point2<f32>>) {
        let mut v1: Option<Point2<f32>> = None;
        for v2 in vertices {
            if let Some(v1) = v1 {
                self.vb.push(Vertex {
                    pos: [v1.x, v1.y],
                    color,
                });
                self.vb.push(Vertex {
                    pos: [v2.x, v2.y],
                    color,
                });
            }
            v1 = Some(v2);
        }
    }
}
