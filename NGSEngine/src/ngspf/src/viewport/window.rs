//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Window node.
use refeq::RefEqArc;
use enumflags::BitFlags;
use cgmath::Vector2;
use context::{Context, KeyedProperty, NodeRef, PropertyAccessor, KeyedPropertyAccessor,
              RoPropertyAccessor, RefPropertyAccessor};

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumFlags)]
    #[repr(u8)]
    pub enum WindowFlagsBit {
        Resizable = 0b01,
        Borderless = 0b10,
        Transparent = 0b100,
    }
}

pub use self::flags::WindowFlagsBit;

pub type WindowFlags = BitFlags<WindowFlagsBit>;

/// Factory type of `WindowRef`.
#[derive(Debug, Clone)]
pub struct WindowBuilder {
    flags: WindowFlags,
    size: Vector2<f32>,
    child: Option<NodeRef>,
    title: String,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            flags: WindowFlags::empty(),
            size: Vector2::new(640f32, 480f32),
            child: None,
            title: "NgsPF Window".to_owned(),
        }
    }

    pub fn flags(self, flags: WindowFlags) -> Self {
        Self { flags, ..self }
    }

    pub fn size(self, size: Vector2<f32>) -> Self {
        Self { size, ..self }
    }

    pub fn child(self, child: Option<NodeRef>) -> Self {
        Self { child, ..self }
    }

    pub fn title<T: Into<String>>(self, title: T) -> Self {
        Self {
            title: title.into(),
            ..self
        }
    }

    pub fn build(self, context: &Context) -> WindowRef {
        WindowRef(RefEqArc::new(Window {
            flags: self.flags,
            size: KeyedProperty::new(context, self.size),
            child: KeyedProperty::new(context, self.child),
            title: KeyedProperty::new(context, self.title),
        }))
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(super) struct Window {
    pub flags: WindowFlags,
    pub size: KeyedProperty<Vector2<f32>>,
    pub child: KeyedProperty<Option<NodeRef>>,
    pub title: KeyedProperty<String>,
}

/// Reference to a window node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowRef(RefEqArc<Window>);

impl WindowRef {
    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(self.0)
    }

    pub fn flags<'a>(&'a self) -> impl RoPropertyAccessor<WindowFlags> + 'a {
        RefPropertyAccessor::new(&self.0.flags)
    }

    pub fn size<'a>(&'a self) -> impl PropertyAccessor<Vector2<f32>> + 'a {
        fn select(this: &RefEqArc<Window>) -> &KeyedProperty<Vector2<f32>> {
            &this.size
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn child<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &RefEqArc<Window>) -> &KeyedProperty<Option<NodeRef>> {
            &this.child
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn title<'a>(&'a self) -> impl PropertyAccessor<String> + 'a {
        fn select(this: &RefEqArc<Window>) -> &KeyedProperty<String> {
            &this.title
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}
