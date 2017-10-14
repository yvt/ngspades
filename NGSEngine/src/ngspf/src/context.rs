//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::{ops, fmt};
use std::collections::{HashMap, hash_map};
use uniqueid::ProcessUniqueId;
use arclock::{ArcLock, ArcLockGuard};
use tokenlock::{TokenLock, TokenRef, Token};

#[derive(Debug)]
pub struct Context {
    producer_frame: ArcLock<ProducerFrameInner>,
    presenter_frame: ArcLock<PresenterFrameInner>,
    changelog: Mutex<Changelog>,
    producer_token_ref: TokenRef,
    presenter_token_ref: TokenRef,
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
    pub fn new() -> Self {
        let producer_token = Token::new();
        let presenter_token = Token::new();
        Self {
            producer_token_ref: TokenRef::from(&producer_token),
            presenter_token_ref: TokenRef::from(&presenter_token),
            producer_frame: ArcLock::new(ProducerFrameInner {
                changeset: HashMap::new(),
                producer_token,
            }),
            presenter_frame: ArcLock::new(PresenterFrameInner { presenter_token }),
            changelog: Mutex::default(),
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

    /// Finalize the current frame for presentation.
    ///
    /// If you have a lock on the current frame, it must be unlocked first (by
    /// dropping `ProducerFrame`). It does not wait until it is unlocked because
    /// doing so has a possibility of a deadlock, which only can happen as a
    /// result of a programming error.
    pub fn commit(&self) -> Result<(), ContextError> {
        use std::mem::swap;
        let mut frame: ArcLockGuard<ProducerFrameInner> =
            self.producer_frame.try_lock().map_err(
                |_| ContextError::LockFailed,
            )?;

        let mut changelog = self.changelog.lock().unwrap();

        let mut changeset = HashMap::with_capacity(frame.changeset.len() * 2);
        swap(&mut changeset, &mut frame.changeset);
        changelog.changesets.push(changeset);

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
        let frame_inner: ArcLockGuard<PresenterFrameInner> =
            self.presenter_frame.try_lock().map_err(
                |_| ContextError::LockFailed,
            )?;

        let mut frame = PresenterFrame(frame_inner);

        // Apply pending changes
        let mut changelog = self.changelog.lock().unwrap();

        for mut changeset in changelog.changesets.drain(..) {
            for (_, mut update) in changeset.drain() {
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
    changeset: HashMap<PropInstId, Box<Update>>,
    producer_token: Token,
}

#[derive(Debug)]
struct PresenterFrameInner {
    presenter_token: Token,
}

#[derive(Debug, Default)]
struct Changelog {
    changesets: Vec<HashMap<PropInstId, Box<Update>>>,
}

/// Reference to a node.
#[derive(Clone)]
pub struct NodeRef(pub Arc<Any + Sync + Send>);

impl fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NodeRef").finish()
    }
}

/// Represents an immutable set of nodes.
struct Group {
    nodes: Vec<NodeRef>,
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Group").finish()
    }
}

/// Reference to a group node, which represents an immutable set of nodes.
#[derive(Debug, Clone)]
pub struct GroupRef(Arc<Group>);

impl GroupRef {
    pub fn new<T: IntoIterator<Item = NodeRef>>(nodes: T) -> Self {
        GroupRef(Arc::new(Group { nodes: nodes.into_iter().collect() }))
    }

    pub fn into_node_ref(self) -> NodeRef {
        NodeRef(self.0)
    }
}

/// Iterate through non-group nodes reachable from a given root node.
pub fn for_each_node<T: FnMut(&NodeRef)>(root: &NodeRef, mut cb: T) {
    fn inner<T: FnMut(&NodeRef)>(root: &NodeRef, cb: &mut T) {
        if let Some(group) = Any::downcast_ref::<Group>(root) {
            for node in group.nodes.iter() {
                inner(node, cb)
            }
        } else {
            cb(root);
        }
    }
    inner(root, &mut cb);
}

/// Property instance ID.
///
/// Must be unique for every *instance* of a property.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct PropInstId(ProcessUniqueId);

impl PropInstId {
    pub fn new() -> Self {
        PropInstId(ProcessUniqueId::new())
    }
}

trait Update: Send + Sync + fmt::Debug {
    fn apply(&mut self, frame: &mut PresenterFrame);
    fn as_any_mut(&mut self) -> &mut (Any + Sync + Send);
}

impl ProducerFrame {
    pub fn record_keyed_update<T, F, FF>(&mut self, prop_id: PropInstId, value: T, trans_fn: FF)
    where
        T: Sync + Send + 'static,
        FF: FnOnce() -> F,
        F: FnOnce(&mut PresenterFrame, T) + 'static + Sync + Send,
    {
        match self.0.changeset.entry(prop_id) {
            hash_map::Entry::Occupied(mut e) => {
                if let Some(updater) = Any::downcast_mut::<KeyedUpdate<T, F>>(
                    e.get_mut().as_any_mut(),
                )
                {
                    updater.0.as_mut().unwrap().0 = value;
                    return;
                }
                e.insert(Box::new(KeyedUpdate(Some((value, trans_fn())))));
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(Box::new(KeyedUpdate(Some((value, trans_fn())))));
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
#[derive(Debug)]
pub struct Property<T> {
    presenter_data: WoProperty<T>,
    producer_data: TokenLock<T>,
}

impl<T> WoProperty<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self { presenter_data: TokenLock::new(context.presenter_token_ref.clone(), x) }
    }

    pub fn write_presenter(&self, frame: &mut PresenterFrame) -> Result<&mut T, PropertyError> {
        self.presenter_data
            .write(&mut frame.0.presenter_token)
            .ok_or(PropertyError::InvalidContext)
    }

    pub fn read_presenter(&self, frame: &PresenterFrame) -> Result<&T, PropertyError> {
        self.presenter_data.read(&frame.0.presenter_token).ok_or(
            PropertyError::InvalidContext,
        )
    }
}

impl<T: Clone> Property<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            presenter_data: WoProperty::new(context, x.clone()),
            producer_data: TokenLock::new(context.producer_token_ref.clone(), x),
        }
    }

    pub fn write_producer(&self, frame: &mut ProducerFrame) -> Result<&mut T, PropertyError> {
        self.producer_data
            .write(&mut frame.0.producer_token)
            .ok_or(PropertyError::InvalidContext)
    }

    pub fn read_producer(&self, frame: &ProducerFrame) -> Result<&T, PropertyError> {
        self.producer_data.read(&frame.0.producer_token).ok_or(
            PropertyError::InvalidContext,
        )
    }
}

impl<T: Clone> ops::Deref for Property<T> {
    type Target = WoProperty<T>;

    fn deref(&self) -> &Self::Target {
        &self.presenter_data
    }
}

/// `Property` with an unique property instance ID (`PropInstId`).
#[derive(Debug)]
pub struct KeyedProperty<T> {
    id: PropInstId,
    property: Property<T>,
}

impl<T: Clone> KeyedProperty<T> {
    pub fn new(context: &Context, x: T) -> Self {
        Self {
            id: PropInstId::new(),
            property: Property::new(context, x),
        }
    }
}

impl<T> KeyedProperty<T> {
    pub fn id(&self) -> PropInstId {
        self.id
    }
}

impl<T> ops::Deref for KeyedProperty<T> {
    type Target = Property<T>;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

/// Dynamic property accessor.
///
/// Property accessors provide an easy way to access and modify properties of
/// nodes. They record a changeset to the frame automatically when updating a
/// property value.
///
/// See the documentation of [`KeyedPropertyAccessor`] for the usage.
///
/// [`KeyedPropertyAccessor`]: struct.KeyedPropertyAccessor.html
pub trait PropertyAccessor<T> {
    fn get(&self, frame: &ProducerFrame) -> Result<T, PropertyError>
    where
        T: Clone,
    {
        self.get_ref(frame).map(T::clone)
    }
    fn get_ref(&self, frame: &ProducerFrame) -> Result<&T, PropertyError>;
    fn set(&self, frame: &mut ProducerFrame, new_value: T) -> Result<(), PropertyError>;
}

/// Dynamic property accessor to a `KeyedProperty`.
///
/// # Examples
///
///     #![feature(conservative_impl_trait)]
///     use ngspf::{PropertyAccessor, KeyedPropertyAccessor, KeyedProperty,
///         ProducerFrame};
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

impl<'a, T, C, F> PropertyAccessor<T> for KeyedPropertyAccessor<'a, C, F>
where
    C: 'static + Clone + Sync + Send,
    F: 'static + Clone + Sync + Send + for<'r> Fn(&'r C) -> &'r KeyedProperty<T>,
    T: 'static + Clone + Send + Sync,
{
    fn get_ref(&self, frame: &ProducerFrame) -> Result<&T, PropertyError> {
        (self.selector)(self.container).read_producer(frame)
    }

    fn set(&self, frame: &mut ProducerFrame, new_value: T) -> Result<(), PropertyError> {
        *(self.selector)(self.container).write_producer(frame)? = new_value.clone();

        frame.record_keyed_update((self.selector)(self.container).id(), new_value, move || {
            let c = self.container.clone();
            let s = self.selector.clone();
            move |frame, value| {
                *s(&c).write_presenter(frame).unwrap() = value;
            }
        });

        Ok(())
    }
}
