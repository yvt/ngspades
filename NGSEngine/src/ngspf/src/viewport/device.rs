//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, RwLock, Mutex};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::{fmt, hash};

use gfx;
use gfx::prelude::*;

pub struct WorkspaceDevice<B: Backend> {
    libraries: RwLock<LibraryMap>,
    objects: DeviceObjects<B>,
}

impl<B: Backend> fmt::Debug for WorkspaceDevice<B> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("WorkspaceDevice")
            .field("libraries", &self.libraries)
            .field("objects", &self.objects)
            .finish()
    }
}

impl<B: Backend> WorkspaceDevice<B> {
    pub(super) fn new(
        gfx_device: Arc<B::Device>,
    ) -> Result<Self, gfx::core::GenericError> {
        let objects = DeviceObjects {
            heap: Arc::new(Mutex::new(gfx_device.factory().make_universal_heap()?)),
            gfx_device,
        };
        Ok(Self {
            libraries: RwLock::new(LibraryMap::new()),
            objects,
        })
    }

    pub fn objects(&self) -> &DeviceObjects<B> {
        &self.objects
    }

    pub fn get_library<T: Library<B>>(this: &Arc<Self>, library: &T) -> Arc<T::Instance> {
        if let Some(inst) = this.libraries.read().unwrap().get(library).cloned() {
            return inst;
        }

        let inst = library.make_instance(this);

        this.libraries
            .write()
            .unwrap()
            .get_or_create(library, || inst)
            .clone()
    }
}

/// Dictionary of `Library::Instance`s.
///
/// Each entry contains the type ID of `T: Library` as its key and a boxed
/// `HashMap<T::LibraryId, Arc<T:Instance>>` as its value.
#[derive(Debug)]
struct LibraryMap(HashMap<TypeId, Box<Any>>);

impl LibraryMap {
    fn new() -> Self {
        LibraryMap(HashMap::new())
    }

    fn get<B, T>(&self, library: &T) -> Option<&Arc<T::Instance>>
    where
        B: Backend,
        T: Library<B>
    {
        let type_id = TypeId::of::<T>();
        self.0.get(&type_id).and_then(|boxed_tlm| {
            let tlm: &HashMap<T::LibraryId, Arc<T::Instance>> = boxed_tlm.downcast_ref().unwrap();
            tlm.get(&library.id())
        })
    }

    fn get_or_create<B, T, F>(&mut self, library: &T, factory: F) -> &Arc<T::Instance>
    where
        B: Backend,
        T: Library<B>,
        F: FnOnce() -> T::Instance,
    {
        let type_id = TypeId::of::<T>();
        let boxed_tlm = self.0.entry(type_id).or_insert_with(|| {
            Box::new(HashMap::<T::LibraryId, Arc<T::Instance>>::new())
        });
        let tlm: &mut HashMap<T::LibraryId, Arc<T::Instance>> = boxed_tlm.downcast_mut().unwrap();
        tlm.entry(library.id()).or_insert_with(
            || Arc::new(factory()),
        )
    }
}

/// NgsGFX objects associated with a certain NgsGFX device.
pub struct DeviceObjects<B: Backend> {
    gfx_device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
}

impl<B: Backend> fmt::Debug for DeviceObjects<B> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("DeviceObjects")
            .field("gfx_device", &self.gfx_device)
            .field("heap", &self.heap)
            .finish()
    }
}

impl<B: Backend> DeviceObjects<B> {
    pub fn gfx_device(&self) -> &Arc<B::Device> {
        &self.gfx_device
    }

    pub fn heap(&self) -> &Arc<Mutex<B::UniversalHeap>> {
        &self.heap
    }
}

pub trait Library<B: Backend>: Any + fmt::Debug {
    /// Identifier used to distingish multiple instances of this `Library`.
    type LibraryId: 'static + hash::Hash + Eq + fmt::Debug;

    /// Runtime data type associated with a specific `Device` and `Library`.
    type Instance: 'static + fmt::Debug;

    /// Get the `LibraryId` of the `Library`.
    fn id(&self) -> Self::LibraryId;

    /// Construct a `Instance` for a specific `Device`.
    fn make_instance(&self, device: &Arc<WorkspaceDevice<B>>) -> Self::Instance;
}