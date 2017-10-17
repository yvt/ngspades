//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Layer node.
use std::sync::Arc;
use enumflags::BitFlags;
use cgmath::Matrix4;
use cgmath::prelude::*;
use refeq::RefEqArc;
use context::{Context, KeyedProperty, NodeRef, PropertyAccessor, KeyedPropertyAccessor};
use super::ImageRef;

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumFlags)]
    #[repr(u8)]
    pub enum LayerFlagsBit {
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
    child: Option<NodeRef>,
}

impl LayerBuilder {
    pub fn new() -> Self {
        Self {
            flags: LayerFlags::empty(),
            transform: Matrix4::identity(),
            opacity: 1.0,
            contents: LayerContents::Empty,
            child: None,
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

    pub fn child(self, child: Option<NodeRef>) -> Self {
        Self { child, ..self }
    }

    pub fn build(self, context: &Context) -> LayerRef {
        LayerRef(Arc::new(Layer {
            flags: KeyedProperty::new(context, self.flags),
            transform: KeyedProperty::new(context, self.transform),
            opacity: KeyedProperty::new(context, self.opacity),
            contents: KeyedProperty::new(context, self.contents),
            child: KeyedProperty::new(context, self.child),
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
    pub child: KeyedProperty<Option<NodeRef>>,
}

#[derive(Debug, Clone)]
pub enum LayerContents {
    /// The layer does not have contents by itself.
    Empty,
    /// Specifies to use a given `Image` as the layer contents.
    Image(ImageRef),
    // TODO
    // Generated(()),
    // BackDrop,
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

    pub fn contents<'a>(&'a self) -> impl PropertyAccessor<LayerContents> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<LayerContents> {
            &this.contents
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn child<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &Arc<Layer>) -> &KeyedProperty<Option<NodeRef>> {
            &this.child
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}
