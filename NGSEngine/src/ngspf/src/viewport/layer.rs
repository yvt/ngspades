//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Layer node.
use std::sync::Arc;

use enumflags::BitFlags;
use cgmath::{Matrix4, Point2};
use cgmath::prelude::*;
use refeq::RefEqArc;

use ngsbase::Box2;
use ngsbase::prelude::*;
use context::{Context, KeyedProperty, NodeRef, PropertyAccessor, KeyedPropertyAccessor};
use super::{ImageRef, Port};

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumFlags)]
    #[repr(u8)]
    pub enum LayerFlagsBit {
        /// Instructs to rasterize the contents of the layer.
        ///
        /// When this flag is specified, the contents (including its children)
        /// of the layer is rendered as a raster image and then composied to
        /// the parent image. This flag is required to enable the following
        /// composition features:
        ///
        ///  - Filters (TODO)
        ///  - Layer mask (`mask` property)
        ///
        /// The bounding rectangle of the rasterized image is defined by the
        /// `bounds` property.
        FlattenContents = 0b1,
    }
}

pub use self::flags::LayerFlagsBit;

pub type LayerFlags = BitFlags<LayerFlagsBit>;

/// Factory type of `LayerRef`.
#[derive(Debug, Clone)]
pub struct LayerBuilder {
    flags: LayerFlags,
    transform: Matrix4<f32>,
    opacity: f32,
    contents: LayerContents,
    bounds: Box2<f32>,
    child: Option<NodeRef>,
    mask: Option<NodeRef>,
}

impl LayerBuilder {
    pub fn new() -> Self {
        Self {
            flags: LayerFlags::empty(),
            transform: Matrix4::identity(),
            opacity: 1.0,
            contents: LayerContents::Empty,
            bounds: Box2::new(Point2::origin(), Point2::origin()),
            child: None,
            mask: None,
        }
    }

    pub fn flags(self, flags: LayerFlags) -> Self {
        Self { flags, ..self }
    }

    pub fn transform(self, transform: Matrix4<f32>) -> Self {
        Self { transform, ..self }
    }

    pub fn opacity(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }

    pub fn contents(self, contents: LayerContents) -> Self {
        Self { contents, ..self }
    }

    pub fn bounds(self, bounds: Box2<f32>) -> Self {
        Self { bounds, ..self }
    }

    pub fn child(self, child: Option<NodeRef>) -> Self {
        Self { child, ..self }
    }

    pub fn mask(self, mask: Option<NodeRef>) -> Self {
        Self { mask, ..self }
    }

    pub fn build(self, context: &Context) -> LayerRef {
        LayerRef(Arc::new(Layer {
            flags: KeyedProperty::new(context, self.flags),
            transform: KeyedProperty::new(context, self.transform),
            opacity: KeyedProperty::new(context, self.opacity),
            contents: KeyedProperty::new(context, self.contents),
            bounds: KeyedProperty::new(context, self.bounds),
            child: KeyedProperty::new(context, self.child),
            mask: KeyedProperty::new(context, self.mask),
        }))
    }
}

impl Default for LayerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(super) struct Layer {
    pub flags: KeyedProperty<LayerFlags>,
    pub transform: KeyedProperty<Matrix4<f32>>,
    pub opacity: KeyedProperty<f32>,
    pub contents: KeyedProperty<LayerContents>,
    pub bounds: KeyedProperty<Box2<f32>>,
    pub child: KeyedProperty<Option<NodeRef>>,
    pub mask: KeyedProperty<Option<NodeRef>>,
}

#[derive(Debug, Clone)]
pub enum LayerContents {
    /// The layer does not have contents by itself.
    Empty,

    /// Specifies to use a given `Image` as the layer contents.
    Image {
        image: ImageRef,
        source: Box2<f32>,
        wrap_mode: ImageWrapMode,
    },

    /// Specifies to use a given `Port` to generate the layer contents.
    Port(Arc<Port>),

    /// Copies contents from the contents of layers with lower Z order in the
    /// nearest rasterization context (root or a layer with `FlattenContents`).
    ///
    /// This layer must have the `FlattenContents` attribute.
    BackDrop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageWrapMode {
    Repeat,
    Clamp,
}

/// Reference to a layer node.
#[derive(Debug, Clone)]
pub struct LayerRef(Arc<Layer>);

impl LayerRef {
    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(RefEqArc::from_arc(self.0))
    }

    pub fn flags<'a>(&'a self) -> impl PropertyAccessor<LayerFlags> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<LayerFlags> {
            &this.flags
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn transform<'a>(&'a self) -> impl PropertyAccessor<Matrix4<f32>> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<Matrix4<f32>> {
            &this.transform
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn opacity<'a>(&'a self) -> impl PropertyAccessor<f32> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<f32> {
            &this.opacity
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    /// Set or retrieve the contents of the layer.
    pub fn contents<'a>(&'a self) -> impl PropertyAccessor<LayerContents> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<LayerContents> {
            &this.contents
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    /// Set or retrieve the bounding rectangle of the contents or an intermediate
    /// raster image (if `FlattenContents` is set).
    pub fn bounds<'a>(&'a self) -> impl PropertyAccessor<Box2<f32>> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<Box2<f32>> {
            &this.bounds
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    /// Set or retrieve the child layer(s) of the layer.
    pub fn child<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<Option<NodeRef>> {
            &this.child
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    /// Set or retrieve the mask image for this layer.
    ///
    /// To enable the mask, both of this layer and the mask have the
    /// `FlattenContents` attribute.
    pub fn mask<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<Option<NodeRef>> {
            &this.mask
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}
