//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Context API of [NgsPF] provides a basic infrastructure for the
//! producer-presenter communication.
//!
//! [NgsPF]: ../ngspf/index.html
//!
//! ## Property Accessor
//!
//! Property accessors provide an easy way to access and modify properties of
//! nodes. They automatically record a changeset to the frame whenever a
//! property value is updated.
//!
//! See the documentation of [`KeyedPropertyAccessor`] for the usage.
//!
//! [`KeyedPropertyAccessor`]: struct.KeyedPropertyAccessor.html
extern crate arclock;
extern crate ngsbase;
extern crate refeq;
extern crate tokenlock;

mod handler;

use std::any::Any;
use std::sync::Mutex;
use std::{borrow, fmt, hash, ops};
use refeq::RefEqArc;
use arclock::{ArcLock, ArcLockGuard};
use tokenlock::{Token, TokenLock, TokenRef};

/// Maintains a single timeline of node property modifications.
#[derive(Debug)]
pub struct Context {
    producer_frame: ArcLock<ProducerFrameInner>,
    presenter_frame: ArcLock<PresenterFrameInner>,
    changelog: Mutex<Changelog>,
    producer_token_ref: TokenRef,
    presenter_token_ref: TokenRef,
    on_commit: Mutex<handler::CommitHandlerList>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ContextError {
    /// Could not acquire a lock on the current frame.
    LockFailed,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum PropertyError {
    InvalidContext,
}

impl Context {
    /// Construct a `Context`.
    pub fn new() -> Self {
        let producer_token = Token::new();
        let presenter_token = Token::new();
        Self {
            producer_token_ref: TokenRef::from(&producer_token),
            presenter_token_ref: TokenRef::from(&presenter_token),
            producer_frame: ArcLock::new(ProducerFrameInner {
                changeset: Vec::new(),
                frame_id: 0,
                producer_token,
            }),
            presenter_frame: ArcLock::new(PresenterFrameInner { presenter_token }),
            changelog: Mutex::default(),
            on_commit: Mutex::new(handler::CommitHandlerList::new()),
        }
    }

    /// Acquire a lock on the current frame of `Context` for the producer access.
    ///
    /// Returns `None` if it is already locked. It does not wait until it is
    /// unlocked because doing so has a possibility of a deadlock, which only
    /// can happen as a result of a programming error.
    pub fn lock_producer_frame(&self) -> Result<ProducerFrame, ContextError> {
        self.producer_frame
            .try_lock()
            .map_err(|_| ContextError::LockFailed)
            .map(ProducerFrame)
    }

    pub fn num_pending_frames(&self) -> usize {
        let changelog = self.changelog.lock().unwrap();
        changelog.changesets.len()
    }

    /// Register a commit handler.
    pub fn on_commit<F: FnMut() + Send + 'static>(&self, handler: F) {
        self.on_commit.lock().unwrap().push(handler);
    }

    /// Finalize the current frame for presentation.
    ///
    /// If you have a lock on the current frame, it must be unlocked first (by
    /// dropping `ProducerFrame`). It does not wait until it is unlocked because
    /// doing so has a possibility of a deadlock, which only can happen as a
    /// result of a programming error.
    ///
    /// **Panics** if too many frames were generated (> `2^64`) during the
    /// lifetime of the `Context`.
    pub fn commit(&self) -> Result<(), ContextError> {
        {
            use std::mem::swap;
            let mut frame: ArcLockGuard<ProducerFrameInner> = self.producer_frame
                .try_lock()
                .map_err(|_| ContextError::LockFailed)?;

            frame.frame_id = frame.frame_id.checked_add(1).expect("frame ID overflow");

            let mut changelog = self.changelog.lock().unwrap();

            let mut changeset = Vec::with_capacity(frame.changeset.len() * 2);
            swap(&mut changeset, &mut frame.changeset);
            changelog.changesets.push(changeset);
        }

        self.on_commit.lock().unwrap().emit();

        Ok(())
    }

    /// Acquire a lock on `Context` for the presenter access.
    ///
    /// Returns `None` if it is already locked. It does not wait until it is
    /// unlocked because doing so has a possibility of a deadlock, which only
    /// can happen as a result of a programming error.
    ///
    /// If locking succeeds, it first applies all changes commited by the
    /// producer so far.
    pub fn lock_presenter_frame(&self) -> Result<PresenterFrame, ContextError> {
        let frame_inner: ArcLockGuard<PresenterFrameInner> = self.presenter_frame
            .try_lock()
            .map_err(|_| ContextError::LockFailed)?;

        let mut frame = PresenterFrame(frame_inner);

        // Apply pending changes
        let mut changelog = self.changelog.lock().unwrap();

        for mut changeset in changelog.changesets.drain(..) {
            for mut update in changeset.drain(..) {
                update.apply(&mut frame);
            }
        }

        Ok(frame)
    }
}

#[derive(Debug)]
pub struct ProducerFrame(ArcLockGuard<ProducerFrameInner>);

#[derive(Debug)]
pub struct PresenterFrame(ArcLockGuard<PresenterFrameInner>);

#[derive(Debug)]
struct ProducerFrameInner {
    changeset: Vec<Box<Update>>,
    producer_token: Token,
    frame_id: u64,
}

#[derive(Debug)]
struct PresenterFrameInner {
    presenter_token: Token,
}

#[derive(Debug, Default)]
struct Changelog {
    changesets: Vec<Vec<Box<Update>>>,
}

/// Marker trait for nodes.
pub trait Node: Any + Sync + Send {}

/// Reference to a node.
#[derive(Clone)]
pub struct NodeRef(pub RefEqArc<Any + Sync + Send>);

impl fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NodeRef").finish()
    }
}

impl NodeRef {
    pub fn downcast_ref<T: Node>(&self) -> Option<&T> {
        Any::downcast_ref(&*self.0)
    }

    /// Iterate through non-group nodes reachable from a given root node via
    /// zero or more group nodes.
    ///
    /// This method returns `Err(x)` as soon as the callback function `cb`
    /// returns `Err(x)`.
    pub fn for_each_node_r<'a, T: FnMut(&'a NodeRef) -> Result<(), E>, E>(
        &'a self,
        mut cb: T,
    ) -> Result<(), E> {
        fn inner<'a, T: FnMut(&'a NodeRef) -> Result<(), E>, E>(
            root: &'a NodeRef,
            cb: &mut T,
        ) -> Result<(), E> {
            if let Some(group) = root.downcast_ref::<Group>() {
                for node in group.nodes.iter() {
                    inner(node, cb)?;
                }
                Ok(())
            } else {
                cb(root)
            }
        }
        inner(self, &mut cb)
    }

    /// Iterate through non-group nodes reachable from a given root node via
    /// zero or more group nodes.
    pub fn for_each_node<'a, T: FnMut(&'a NodeRef)>(&'a self, mut cb: T) {
        self.for_each_node_r::<_, ()>(move |node| {
            cb(node);
            Ok(())
        }).unwrap()
    }

    /// Iterate through nodes of a specific concrete type reachable from a given
    /// root node via zero or more group nodes.
    ///
    /// This method returns `Err(x)` as soon as the callback function `cb`
    /// returns `Err(x)`.
    pub fn for_each_node_of_r<'a, T: Node, F: FnMut(&'a T) -> Result<(), E>, E>(
        &'a self,
        mut cb: F,
    ) -> Result<(), E> {
        self.for_each_node_r(move |node_ref| {
            if let Some(node) = node_ref.downcast_ref() {
                cb(node)
            } else {
                Ok(())
            }
        })
    }

    /// Iterate through nodes of a specific concrete type reachable from a given
    /// root node via zero or more group nodes.
    pub fn for_each_node_of<'a, T: Node, F: FnMut(&'a T)>(&'a self, mut cb: F) {
        self.for_each_node_of_r::<_, _, ()>(move |node| {
            cb(node);
            Ok(())
        }).unwrap()
    }
}

// implementing them using `derive` results in error messages which are
// confusing beyond comprehension
impl PartialEq for NodeRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for NodeRef {}

impl hash::Hash for NodeRef {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

/// Represents an immutable set of nodes.
struct Group {
    nodes: Vec<NodeRef>,
}

impl Node for Group {}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Group").finish()
    }
}

/// Reference to a group node, which represents an immutable set of nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupRef(RefEqArc<Group>);

impl GroupRef {
    pub fn empty() -> Self {
        Self::new(::std::iter::empty())
    }

    pub fn new<T: IntoIterator<Item = NodeRef>>(nodes: T) -> Self {
        GroupRef(RefEqArc::new(Group {
            nodes: nodes.into_iter().collect(),
        }))
    }

    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(self.0)
    }
}

/// Update ID.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct UpdateId {
    frame_id: u64,
    changeset_index: usize,
}

impl UpdateId {
    /// Construct a `UpdateId` that does not correspond to any actual update.
    pub fn new() -> Self {
        Self {
            frame_id: <u64>::max_value(),
            changeset_index: 0,
        }
    }
}

trait Update: Send + Sync + fmt::Debug {
    fn apply(&mut self, frame: &mut PresenterFrame);
    fn as_any_mut(&mut self) -> &mut (Any + Sync + Send);
}

impl ProducerFrame {
    /// Record a update to the frame's changeset and return the identifier of
    /// the update.
    ///
    /// If the given update ID (`last_update`) points a update in the changeset
    /// of the same frame, it will overwrite the previous update and return the
    /// same update ID (and avoid the insertion cost of a update).
    ///
    /// TODO: elaborate
    pub fn record_keyed_update<T, TF, F, FF>(
        &mut self,
        last_update: UpdateId,
        trans_fn: TF,
        update_fn_fac: FF,
    ) -> UpdateId
    where
        T: Sync + Send + 'static,
        TF: FnOnce(Option<T>) -> T,
        FF: FnOnce() -> F,
        F: FnOnce(&mut PresenterFrame, T) + 'static + Sync + Send,
    {
        if self.0.frame_id == last_update.frame_id {
            let ref mut ent = self.0.changeset[last_update.changeset_index];

            if let Some(updater) = Any::downcast_mut::<KeyedUpdate<T, F>>(ent.as_any_mut()) {
                let (old_value, update_fn) = updater.0.take().unwrap();
                updater.0 = Some((trans_fn(Some(old_value)), update_fn));
                return last_update;
            }

            *ent = Box::new(KeyedUpdate(Some((trans_fn(None), update_fn_fac()))));
            last_update
        } else {
            self.0.changeset.push(Box::new(KeyedUpdate(Some((
                trans_fn(None),
                update_fn_fac(),
            )))));

            UpdateId {
                frame_id: self.0.frame_id,
                changeset_index: self.0.changeset.len() - 1,
            }
        }
    }
}

struct KeyedUpdate<T, F>(Option<(T, F)>);

impl<T, F> Update for KeyedUpdate<T, F>
where
    T: Sync + Send + 'static,
    F: FnOnce(&mut PresenterFrame, T) + Sync + Send + 'static,
{
    fn apply(&mut self, frame: &mut PresenterFrame) {
        let inner = self.0.take().expect("KeyedUpdate was used twice");
        inner.1(frame, inner.0);
    }
    fn as_any_mut(&mut self) -> &mut (Any + Sync + Send) {
        self
    }
}

impl<T, F> fmt::Debug for KeyedUpdate<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("KeyedUpdate").finish()
    }
}

/// Dynamic property of a node with write-only access by the producer.
#[derive(Debug)]
pub struct WoProperty<T> {
    presenter_data: TokenLock<T>,
}

/// Dynamic property of a node with read/write access by the producer.
///
/// This is equivalent to `ProducerDataCell` combined with `WoProperty`.
#[derive(Debug)]
pub struct Property<T> {
    presenter_data: WoProperty<T>,
    producer_data: ProducerDataCell<T>,
}

impl<T> WoProperty<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            presenter_data: TokenLock::new(context.presenter_token_ref.clone(), x),
        }
    }

    pub fn write_presenter<'a>(
        &'a self,
        frame: &'a mut PresenterFrame,
    ) -> Result<&'a mut T, PropertyError> {
        self.presenter_data
            .write(&mut frame.0.presenter_token)
            .ok_or(PropertyError::InvalidContext)
    }

    pub fn read_presenter<'a>(&'a self, frame: &'a PresenterFrame) -> Result<&'a T, PropertyError> {
        self.presenter_data
            .read(&frame.0.presenter_token)
            .ok_or(PropertyError::InvalidContext)
    }
}

impl<T: Clone> Property<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            presenter_data: WoProperty::new(context, x.clone()),
            producer_data: ProducerDataCell::new(context, x),
        }
    }
}

impl<T> Property<T> {
    pub fn write_producer<'a>(
        &'a self,
        frame: &'a mut ProducerFrame,
    ) -> Result<&'a mut T, PropertyError> {
        self.producer_data.write_producer(frame)
    }

    pub fn read_producer<'a>(&'a self, frame: &'a ProducerFrame) -> Result<&'a T, PropertyError> {
        self.producer_data.read_producer(frame)
    }
}

impl<T: Clone> ops::Deref for Property<T> {
    type Target = WoProperty<T>;

    fn deref(&self) -> &Self::Target {
        &self.presenter_data
    }
}

/// Cell whose contents only can be manipulated by the producer.
#[derive(Debug)]
pub struct ProducerDataCell<T> {
    data: TokenLock<T>,
}

impl<T> ProducerDataCell<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            data: TokenLock::new(context.producer_token_ref.clone(), x),
        }
    }

    pub fn write_producer<'a>(
        &'a self,
        frame: &'a mut ProducerFrame,
    ) -> Result<&'a mut T, PropertyError> {
        self.data
            .write(&mut frame.0.producer_token)
            .ok_or(PropertyError::InvalidContext)
    }

    pub fn read_producer<'a>(&'a self, frame: &'a ProducerFrame) -> Result<&'a T, PropertyError> {
        self.data
            .read(&frame.0.producer_token)
            .ok_or(PropertyError::InvalidContext)
    }
}

/// `Property` with an internally managed `UpdateId`.
///
/// This is equivalent to `ProducerDataCell<UpdateId>` combined with `Property<T>`
/// but adds some space/performance optimization.
#[derive(Debug)]
pub struct KeyedProperty<T> {
    // Merge `TokenLock<T>` and `TokenLock<UpdateId>` for performance boost
    producer_data: ProducerDataCell<(T, UpdateId)>,
    property: WoProperty<T>,
}

impl<T: Clone> KeyedProperty<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            producer_data: ProducerDataCell::new(context, (x.clone(), UpdateId::new())),
            property: WoProperty::new(context, x),
        }
    }
}

impl<T> KeyedProperty<T> {
    pub fn write_producer<'a>(
        &'a self,
        frame: &'a mut ProducerFrame,
    ) -> Result<&'a mut T, PropertyError> {
        self.producer_data.write_producer(frame).map(|d| &mut d.0)
    }

    pub fn read_producer<'a>(&'a self, frame: &'a ProducerFrame) -> Result<&'a T, PropertyError> {
        self.producer_data.read_producer(frame).map(|d| &d.0)
    }
}

impl<T> ops::Deref for KeyedProperty<T> {
    type Target = WoProperty<T>;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

/// Dynamic property accessor for read access by the producer.
pub trait PropertyProducerRead<T> {
    fn get(&self, frame: &ProducerFrame) -> Result<T, PropertyError>
    where
        T: Clone,
    {
        self.get_ref(frame).map(T::clone)
    }
    fn get_ref<'a>(&'a self, frame: &'a ProducerFrame) -> Result<&'a T, PropertyError>;
}

/// Dynamic property accessor for write access by the producer.
pub trait PropertyProducerWrite<T> {
    fn set(&self, frame: &mut ProducerFrame, new_value: T) -> Result<(), PropertyError>;
}

/// Dynamic property accessor for read access by the presenter.
pub trait PropertyPresenterRead<T> {
    fn get_presenter(&self, frame: &PresenterFrame) -> Result<T, PropertyError>
    where
        T: Clone,
    {
        self.get_presenter_ref(frame).map(T::clone)
    }
    fn get_presenter_ref<'a>(&'a self, frame: &'a PresenterFrame) -> Result<&'a T, PropertyError>;
}

/// Dynamic property accessor traits.
pub trait PropertyAccessor<T>
    : PropertyProducerRead<T> + PropertyProducerWrite<T> + PropertyPresenterRead<T>
    {
}

/// Read-only dynamic property accessor traits.
pub trait RoPropertyAccessor<T>
    : PropertyProducerRead<T> + PropertyPresenterRead<T> {
}

/// Dynamic property accessor for `KeyedProperty`.
///
/// # Examples
///
///     #![feature(conservative_impl_trait)]
///     use ngspf::context::{KeyedPropertyAccessor, KeyedProperty, ProducerFrame};
///     use ngspf::context::{PropertyAccessor, PropertyProducerWrite};
///     use std::sync::Arc;
///
///     struct Pegasus {
///         derp: KeyedProperty<f32>,
///     }
///
///     struct PegasusRef(Arc<Pegasus>);
///
///     impl PegasusRef {
///         pub fn derp<'a>(&'a self) -> impl PropertyAccessor<f32> + 'a {
///             // work-around for https://github.com/rust-lang/rust/issues/23501
///             fn select(this: &Arc<Pegasus>) -> &KeyedProperty<f32> {
///                 &this.derp
///             }
///             KeyedPropertyAccessor::new(&self.0, select)
///         }
///     }
///
///     fn foo(frame: &mut ProducerFrame, pegasus: &PegasusRef) {
///         pegasus.derp().set(frame, 4.0).unwrap();
///     }
///
#[derive(Debug)]
pub struct KeyedPropertyAccessor<'a, C: 'static, F: 'static> {
    container: &'a C,
    selector: F,
}

impl<'a, C: 'static, F: 'static> KeyedPropertyAccessor<'a, C, F> {
    pub fn new(container: &'a C, selector: F) -> Self {
        Self {
            container,
            selector,
        }
    }
}

impl<'a, T, C, F> PropertyProducerRead<T> for KeyedPropertyAccessor<'a, C, F>
where
    F: for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
{
    fn get_ref<'b>(&'b self, frame: &'b ProducerFrame) -> Result<&'b T, PropertyError> {
        (self.selector)(self.container).read_producer(frame)
    }
}

impl<'a, T, C, F> PropertyPresenterRead<T> for KeyedPropertyAccessor<'a, C, F>
where
    F: for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
{
    fn get_presenter_ref<'b>(&'b self, frame: &'b PresenterFrame) -> Result<&'b T, PropertyError> {
        (self.selector)(self.container).read_presenter(frame)
    }
}

impl<'a, T, C, F> PropertyProducerWrite<T> for KeyedPropertyAccessor<'a, C, F>
where
    C: 'static + Clone + Sync + Send,
    F: 'static + Clone + Sync + Send + for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
    T: 'static + Clone + Sync + Send,
{
    fn set(&self, frame: &mut ProducerFrame, new_value: T) -> Result<(), PropertyError> {
        let prop = (self.selector)(self.container);
        *prop.write_producer(frame)? = new_value.clone();

        let update_id = prop.producer_data.read_producer(frame)?.1;

        let new_id = frame.record_keyed_update(
            update_id,
            |_| new_value,
            || {
                let c = self.container.clone();
                let s = self.selector.clone();
                move |frame, value| {
                    *s(&c).write_presenter(frame).unwrap() = value;
                }
            },
        );

        prop.producer_data.write_producer(frame)?.1 = new_id;

        Ok(())
    }
}

impl<'a, T, C, F> RoPropertyAccessor<T> for KeyedPropertyAccessor<'a, C, F>
where
    F: for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
{
}

impl<'a, T, C, F> PropertyAccessor<T> for KeyedPropertyAccessor<'a, C, F>
where
    C: 'static + Clone + Sync + Send,
    F: 'static + Clone + Sync + Send + for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
    T: 'static + Clone + Sync + Send,
{
}

/// Dynamic property accessor for read-only properties.
///
/// This type implements the same traits except `PropertyProducerWrite` as
/// `KeyedPropertyAccessor` does.
///
/// # Examples
///
///     #![feature(conservative_impl_trait)]
///     use ngspf::context::{RefPropertyAccessor, ProducerFrame};
///     use ngspf::context::{RoPropertyAccessor, PropertyProducerRead};
///     use std::sync::Arc;
///
///     struct Pegasus {
///         derp: f32,
///     }
///
///     struct PegasusRef(Arc<Pegasus>);
///
///     impl PegasusRef {
///         pub fn derp<'a>(&'a self) -> impl RoPropertyAccessor<f32> + 'a {
///             RefPropertyAccessor::new(&self.0.derp)
///         }
///     }
///
///     fn foo(frame: &ProducerFrame, pegasus: &PegasusRef) -> f32 {
///         pegasus.derp().get(frame).unwrap()
///     }
///
#[derive(Debug, Clone, Copy)]
pub struct RefPropertyAccessor<T>(T);

impl<T> RefPropertyAccessor<T> {
    pub fn new(x: T) -> Self {
        RefPropertyAccessor(x)
    }
}

impl<T, S> PropertyProducerRead<S> for RefPropertyAccessor<T>
where
    T: borrow::Borrow<S>,
{
    fn get_ref<'b>(&'b self, _frame: &'b ProducerFrame) -> Result<&'b S, PropertyError> {
        Ok(self.0.borrow())
    }
}

impl<T, S> PropertyPresenterRead<S> for RefPropertyAccessor<T>
where
    T: borrow::Borrow<S>,
{
    fn get_presenter_ref<'b>(&'b self, _frame: &'b PresenterFrame) -> Result<&'b S, PropertyError> {
        Ok(self.0.borrow())
    }
}

impl<T, S> RoPropertyAccessor<S> for RefPropertyAccessor<T>
where
    T: borrow::Borrow<S>,
{
}

/// The NgsPF prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use {PropertyAccessor, PropertyPresenterRead, PropertyProducerRead, PropertyProducerWrite,
             RoPropertyAccessor};
}
