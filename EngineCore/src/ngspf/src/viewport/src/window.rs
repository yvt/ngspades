//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Window node.
use cgmath::Vector2;
use core::{
    Context, KeyedProperty, KeyedPropertyAccessor, Node, NodeRef, ProducerDataCell, ProducerFrame,
    PropertyAccessor, PropertyError, PropertyProducerWrite, RefPropertyAccessor,
    RoPropertyAccessor, UpdateId, WoProperty,
};
use ngsenumflags::BitFlags;
use refeq::RefEqArc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
#[repr(u8)]
pub enum WindowFlagsBit {
    /// Specifies that the window can be resized by the user.
    Resizable = 0b0001,

    /// Hides the window's decoration (title bar, border, etc.).
    Borderless = 0b0010,

    /// Makes the background of the window transparent.
    Transparent = 0b0100,
}

pub type WindowFlags = BitFlags<WindowFlagsBit>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
#[repr(u8)]
pub(super) enum WindowActionBit {
    ChangeSize = 0b1,
    ChangeTitle = 0b10,
}

/// Factory type of `WindowRef`.
pub struct WindowBuilder {
    flags: WindowFlags,
    size: Vector2<f32>,
    min_size: Option<Vector2<f32>>,
    max_size: Option<Vector2<f32>>,
    child: Option<NodeRef>,
    title: String,
    listener: Option<WindowListener>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            flags: WindowFlags::empty(),
            size: Vector2::new(640f32, 480f32),
            min_size: None,
            max_size: None,
            child: None,
            title: "NgsPF Window".to_owned(),
            listener: None,
        }
    }

    pub fn flags<T: Into<WindowFlags>>(self, flags: T) -> Self {
        Self {
            flags: flags.into(),
            ..self
        }
    }

    pub fn size(self, size: Vector2<f32>) -> Self {
        Self { size, ..self }
    }

    pub fn min_size(self, min_size: Option<Vector2<f32>>) -> Self {
        Self { min_size, ..self }
    }

    pub fn max_size(self, max_size: Option<Vector2<f32>>) -> Self {
        Self { max_size, ..self }
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

    pub fn listener(self, listener: Option<WindowListener>) -> Self {
        Self { listener, ..self }
    }

    pub fn build(self, context: &Context) -> WindowRef {
        WindowRef(RefEqArc::new(Window {
            action: WoProperty::new(context, BitFlags::empty()),

            flags: self.flags,

            size: WoProperty::new(context, self.size),
            min_size: WoProperty::new(context, self.min_size),
            max_size: WoProperty::new(context, self.max_size),
            size_update_id: ProducerDataCell::new(context, UpdateId::new()),

            child: KeyedProperty::new(context, self.child),

            title: WoProperty::new(context, self.title),
            title_update_id: ProducerDataCell::new(context, UpdateId::new()),

            listener: WoProperty::new(context, self.listener),
            listener_update_id: ProducerDataCell::new(context, UpdateId::new()),

            mouse_pos: WoProperty::new(context, None),
        }))
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) struct Window {
    pub action: WoProperty<BitFlags<WindowActionBit>>,

    pub flags: WindowFlags,

    pub size: WoProperty<Vector2<f32>>,
    pub min_size: WoProperty<Option<Vector2<f32>>>,
    pub max_size: WoProperty<Option<Vector2<f32>>>,
    pub size_update_id: ProducerDataCell<UpdateId>,

    pub child: KeyedProperty<Option<NodeRef>>,

    pub title: WoProperty<String>,
    pub title_update_id: ProducerDataCell<UpdateId>,

    pub listener: WoProperty<Option<WindowListener>>,
    pub listener_update_id: ProducerDataCell<UpdateId>,

    // only used by presenter (not exposed to the application)
    pub mouse_pos: WoProperty<Option<MousePosition>>,
}

impl Node for Window {}

/// Reference to a window node.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WindowRef(RefEqArc<Window>);

impl WindowRef {
    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(self.0)
    }

    pub fn flags<'a>(&'a self) -> impl RoPropertyAccessor<WindowFlags> + 'a {
        RefPropertyAccessor::new(&self.0.flags)
    }

    pub fn size<'a>(&'a self) -> impl PropertyProducerWrite<Vector2<f32>> + 'a {
        struct Accessor<'a>(&'a WindowRef);
        impl<'a> PropertyProducerWrite<Vector2<f32>> for Accessor<'a> {
            fn set(
                &self,
                frame: &mut ProducerFrame,
                new_value: Vector2<f32>,
            ) -> Result<(), PropertyError> {
                let update_id = *(self.0).0.size_update_id.read_producer(frame)?;

                let new_id = frame.record_keyed_update(
                    update_id,
                    |_| new_value,
                    || {
                        let c = RefEqArc::clone(&(self.0).0);
                        move |frame, value| {
                            *c.size.write_presenter(frame).unwrap() = value;
                            let a = c.action.write_presenter(frame).unwrap();
                            *a = *a | WindowActionBit::ChangeSize;
                        }
                    },
                );

                *(self.0).0.size_update_id.write_producer(frame)? = new_id;

                Ok(())
            }
        }
        Accessor(self)
    }

    pub fn min_size<'a>(&'a self) -> impl PropertyProducerWrite<Option<Vector2<f32>>> + 'a {
        struct Accessor<'a>(&'a WindowRef);
        impl<'a> PropertyProducerWrite<Option<Vector2<f32>>> for Accessor<'a> {
            fn set(
                &self,
                frame: &mut ProducerFrame,
                new_value: Option<Vector2<f32>>,
            ) -> Result<(), PropertyError> {
                let update_id = *(self.0).0.size_update_id.read_producer(frame)?;

                let new_id = frame.record_keyed_update(
                    update_id,
                    |_| new_value,
                    || {
                        let c = RefEqArc::clone(&(self.0).0);
                        move |frame, value| {
                            *c.min_size.write_presenter(frame).unwrap() = value;
                            let a = c.action.write_presenter(frame).unwrap();
                            *a = *a | WindowActionBit::ChangeSize;
                        }
                    },
                );

                *(self.0).0.size_update_id.write_producer(frame)? = new_id;

                Ok(())
            }
        }
        Accessor(self)
    }

    pub fn max_size<'a>(&'a self) -> impl PropertyProducerWrite<Option<Vector2<f32>>> + 'a {
        struct Accessor<'a>(&'a WindowRef);
        impl<'a> PropertyProducerWrite<Option<Vector2<f32>>> for Accessor<'a> {
            fn set(
                &self,
                frame: &mut ProducerFrame,
                new_value: Option<Vector2<f32>>,
            ) -> Result<(), PropertyError> {
                let update_id = *(self.0).0.size_update_id.read_producer(frame)?;

                let new_id = frame.record_keyed_update(
                    update_id,
                    |_| new_value,
                    || {
                        let c = RefEqArc::clone(&(self.0).0);
                        move |frame, value| {
                            *c.max_size.write_presenter(frame).unwrap() = value;
                            let a = c.action.write_presenter(frame).unwrap();
                            *a = *a | WindowActionBit::ChangeSize;
                        }
                    },
                );

                *(self.0).0.size_update_id.write_producer(frame)? = new_id;

                Ok(())
            }
        }
        Accessor(self)
    }

    pub fn child<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &RefEqArc<Window>) -> &KeyedProperty<Option<NodeRef>> {
            &this.child
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn title<'a>(&'a self) -> impl PropertyProducerWrite<String> + 'a {
        struct Accessor<'a>(&'a WindowRef);
        impl<'a> PropertyProducerWrite<String> for Accessor<'a> {
            fn set(
                &self,
                frame: &mut ProducerFrame,
                new_value: String,
            ) -> Result<(), PropertyError> {
                let update_id = *(self.0).0.title_update_id.read_producer(frame)?;

                let new_id = frame.record_keyed_update(
                    update_id,
                    |_| new_value,
                    || {
                        let c = RefEqArc::clone(&(self.0).0);
                        move |frame, value| {
                            *c.title.write_presenter(frame).unwrap() = value;
                            let a = c.action.write_presenter(frame).unwrap();
                            *a = *a | WindowActionBit::ChangeTitle;
                        }
                    },
                );

                *(self.0).0.title_update_id.write_producer(frame)? = new_id;

                Ok(())
            }
        }
        Accessor(self)
    }

    pub fn listener<'a>(&'a self) -> impl PropertyProducerWrite<Option<WindowListener>> + 'a {
        struct Accessor<'a>(&'a WindowRef);
        impl<'a> PropertyProducerWrite<Option<WindowListener>> for Accessor<'a> {
            fn set(
                &self,
                frame: &mut ProducerFrame,
                new_value: Option<WindowListener>,
            ) -> Result<(), PropertyError> {
                let update_id = *(self.0).0.listener_update_id.read_producer(frame)?;

                let new_id = frame.record_keyed_update(
                    update_id,
                    |_| new_value,
                    || {
                        let c = RefEqArc::clone(&(self.0).0);
                        move |frame, value| {
                            *c.listener.write_presenter(frame).unwrap() = value;
                        }
                    },
                );

                *(self.0).0.listener_update_id.write_producer(frame)? = new_id;

                Ok(())
            }
        }
        Accessor(self)
    }
}

pub type WindowListener = Box<Fn(&WindowEvent) + Send + Sync>;

#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    Resized(Vector2<f32>),
    Moved(Vector2<f32>),
    Close,

    /// The window gained (`true`) or lost (`false`) focus.
    Focused(bool),

    /// A mouse button was pressed or released.
    ///
    /// The third parameter indicates whether the button was pressed (`true`)
    /// or released (`false`).
    MouseButton(MousePosition, MouseButton, bool),

    /// The mouse cursor has moved on the window, or just left the window's
    /// client region (in which case the position is `None`).
    MouseMotion(Option<MousePosition>),

    /// A key was pressed or released.
    ///
    /// The second parameter indicates whether the key was pressed (`true`)
    /// or released (`false`).
    KeyboardInput(VirtualKeyCode, bool, KeyModifierFlags),
}

#[derive(Debug, Clone, Copy)]
pub struct MousePosition {
    /// The mouse cursor's position in device independent pixels relative to
    /// the top-left corner of the window's client region.
    pub client: Vector2<f32>,

    /// The mouse cursor's position in device independent pixels relative to
    /// a point independent to the window's position.
    pub global: Vector2<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

pub use winit::VirtualKeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
#[repr(u8)]
pub enum KeyModifier {
    Shift = 0b0001,
    Control = 0b0010,
    Alt = 0b0100,
    Meta = 0b1000,
}

pub type KeyModifierFlags = BitFlags<KeyModifier>;
