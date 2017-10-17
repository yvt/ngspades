//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Window node.
use refeq::RefEqArc;
use cgmath::Vector2;
use context::{Context, KeyedProperty, NodeRef, PropertyAccessor, KeyedPropertyAccessor};

/// Factory type of `WindowRef`.
#[derive(Debug, Clone)]
pub struct WindowBuilder {
    size: Vector2<f32>,
    child: Option<NodeRef>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            size: Vector2::new(640f32, 480f32),
            child: None,
        }
    }

    pub fn size(self, size: Vector2<f32>) -> Self {
        Self { size, ..self }
    }

    pub fn child(self, child: Option<NodeRef>) -> Self {
        Self { child, ..self }
    }

    pub fn build(self, context: &Context) -> WindowRef {
        WindowRef(RefEqArc::new(Window {
            size: KeyedProperty::new(context, self.size),
            child: KeyedProperty::new(context, self.child),
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
    pub size: KeyedProperty<Vector2<f32>>,
    pub child: KeyedProperty<Option<NodeRef>>,
}

/// Reference to a window node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowRef(RefEqArc<Window>);

impl WindowRef {
    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(self.0)
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
}
