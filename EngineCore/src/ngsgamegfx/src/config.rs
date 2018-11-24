//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Configuration of the rendering engine.
use query_interface::{interfaces, vtable_for, Object, ObjectEq, ObjectHash, ObjectPartialEq};
use std::{fmt::Debug, hash};

/// A metadata of a single configuration option.
pub trait ConfigItem: Send + Sync {
    /// Cast `self` to a trait object `&dyn Object`.
    fn as_object(&self) -> &dyn Object;

    /// Get a reference to the corresponding field of a given `Config`.
    fn get_dyn<'a>(&self, config: &'a Config) -> &'a dyn Object;

    /// Get a mutable reference to the corresponding field of a given `Config`.
    fn get_dyn_mut<'a>(&self, config: &'a mut Config) -> &'a mut dyn Object;
}

impl PartialEq for dyn ConfigItem {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}

impl Eq for dyn ConfigItem {}

impl hash::Hash for dyn ConfigItem {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_object().hash(state)
    }
}

macro_rules! impl_config_item {
    ($name:ty { |config| &config.$field:ident }) => {
        interfaces!($name: ObjectPartialEq, ObjectEq, ObjectHash, Debug);

        impl ConfigItem for $name {
            fn as_object(&self) -> &dyn Object {
                self
            }

            fn get_dyn<'a>(&self, config: &'a Config) -> &'a dyn Object {
                &config.$field
            }
            fn get_dyn_mut<'a>(&self, config: &'a mut Config) -> &'a mut dyn Object {
                &mut config.$field
            }
        }
    };
}

/// The metadata of all configuration options in [`Config`].
pub const CONFIG_ITEMS: &[&dyn ConfigItem] = &[&RenderScale];

/// The metadata of [`Config::render_scale`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RenderScale;
impl_config_item!(RenderScale { |config| &config.render_scale });

/// The configuration of the rendering engine. All configuration options are
/// modifiable at run-time.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub render_scale: f32,
    // TODO
}

impl Default for Config {
    fn default() -> Self {
        Self { render_scale: 1.0 }
    }
}
