//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Resource state tracking.
//!
//! Each (resource, queue) is associated with the following
//! objects:
//!
//!  - One `State`, which represents the last known state of the resource.
//!
//!  - `MAX_NUM_ACTIVE_CMD_BUFFERS` `Op`s (operations), each of which describes
//!    how the `State` will be transformed when the corresponding command buffer
//!    is executed.
//!
use lazy_static::lazy_static;
use snowflake::ProcessUniqueId;
use std::cell::UnsafeCell;

use crate::MAX_NUM_ACTIVE_CMD_BUFFERS;

/// Identifies a queue.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
crate struct QueueId(ProcessUniqueId);

impl QueueId {
    /// Return a dummy `QueueId` value associated with no queue.
    crate fn dummy_value() -> QueueId {
        lazy_static! {
            static ref DUMMY_VALUE: ProcessUniqueId = ProcessUniqueId::new();
        }
        QueueId(*DUMMY_VALUE)
    }

    fn new() -> Self {
        QueueId(ProcessUniqueId::new())
    }
}

/// Represents an exclusive ownership of state data associated with a queue.
#[derive(Debug)]
crate struct Queue {
    queue_id: QueueId,
}

impl Queue {
    /// Return a queue identifier.
    crate fn queue_id(&self) -> QueueId {
        self.queue_id
    }
}

/// Represents a nullable index into a reference table. It always uses a 32-bit
/// integer to reduce memory footprint.
#[derive(Eq, PartialEq, Copy, Clone)]
struct RefTableIndex(u32);

impl RefTableIndex {
    const NONE: Self = RefTableIndex(0xffffffffu32);

    fn get(&self) -> Option<usize> {
        if *self == Self::NONE {
            None
        } else {
            Some(self.0 as usize)
        }
    }
}

impl From<Option<usize>> for RefTableIndex {
    fn from(x: Option<usize>) -> Self {
        if let Some(x) = x {
            if x >= Self::NONE.0 as usize {
                panic!("too many referenced resources");
            }
            RefTableIndex(x as u32)
        } else {
            Self::NONE
        }
    }
}

use std::fmt;
impl fmt::Debug for RefTableIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RefTableIndex").field(&self.get()).finish()
    }
}

/// Represents an exclusive ownership of state data associated with a command
/// buffer.
#[derive(Debug)]
crate struct CmdBuffer {
    queue_id: QueueId,
    index: usize,
}

/// The current tracked state of a resource on a particular queue.
#[derive(Debug)]
crate struct TrackedState<State> {
    queue_id: QueueId,

    /// The last known state. More precisely, this represents the state at the
    /// point of time when the command buffers which were previously submitted
    /// to the queue completed execution.
    latest: UnsafeCell<State>,

    /// Each element at index `i` specifies an index into the reference table
    /// (`RefTable`) of the corresponding command buffer at the index `i`.
    ref_table_indices: [UnsafeCell<RefTableIndex>; MAX_NUM_ACTIVE_CMD_BUFFERS],
}

unsafe impl<State> Sync for TrackedState<State> {}

/// A reference table. Each command buffer should have one for each tracked
/// resource type.
#[derive(Debug)]
crate struct RefTable<Res, Op>(Vec<(Res, Op)>);

/// Represents a handle to a tracked resource associated with a particular queue.
crate trait Resource: Clone {
    type State;

    /// Get the `TrackedState` representing the tracked state of the resource.
    /// Implementor must ensure that a single, identical object is returned
    /// throughout its lifetime.
    fn tracked_state(&self) -> &TrackedState<Self::State>;
}

/// Construct a `Queue` and a set of `CmdBuffer`s for a particular queue.
crate fn new_queue() -> (Queue, Vec<CmdBuffer>) {
    let queue_id = QueueId::new();

    let queue = Queue { queue_id };
    let cmd_buffers = (0..MAX_NUM_ACTIVE_CMD_BUFFERS)
        .map(|index| CmdBuffer { queue_id, index })
        .collect();

    (queue, cmd_buffers)
}

impl<State> TrackedState<State> {
    /// Construct a `TrackedState`.
    crate fn new(queue_id: QueueId, latest: State) -> Self {
        use std::mem::transmute;
        Self {
            queue_id,
            latest: UnsafeCell::new(latest),
            ref_table_indices: unsafe {
                transmute([RefTableIndex::NONE; MAX_NUM_ACTIVE_CMD_BUFFERS])
            },
        }
    }

    /// Get a mutable reference to `latest` of `self`. Panics if the resource is
    /// not associated with a specified queue.
    ///
    /// (It essentially replicates the behavior of `tokenlock`, except that
    /// the key type is different.)
    crate fn latest_mut<'a>(&'a self, queue: &'a mut Queue) -> &'a mut State {
        unsafe {
            assert_eq!(self.queue_id, queue.queue_id, "queue mismatch");
            &mut *self.latest.get()
        }
    }

    /// Get a mutable reference to an element of `ref_table_indices`
    /// corresponding to a given command buffer. Panics if the resource is not
    /// associated with the queue of the command buffer.
    fn ref_table_index_mut<'a>(&'a self, cmd_buffer: &'a mut CmdBuffer) -> &'a mut RefTableIndex {
        unsafe {
            assert_eq!(self.queue_id, cmd_buffer.queue_id, "queue mismatch");
            &mut *self.ref_table_indices.get_unchecked(cmd_buffer.index).get()
        }
    }
}

#[derive(Debug)]
crate struct RefTableEntry<'a, Res: 'a, Op: 'a> {
    crate index: usize,
    crate resource: &'a Res,
    crate op: &'a Op,
}

#[derive(Debug)]
crate struct RefTableEntryMut<'a, Res: 'a, Op: 'a> {
    crate index: usize,
    crate resource: &'a Res,
    crate op: &'a mut Op,
}

impl<Res, Op> Default for RefTable<Res, Op> {
    fn default() -> Self {
        RefTable(Vec::new())
    }
}

impl<Res: Resource, Op: Default> RefTable<Res, Op> {
    crate fn new() -> Self {
        Default::default()
    }

    /// Get the index of the entry describing how the resource state
    /// represented by `res` will be transformed during the execution of
    /// the command buffer represented by `cmd_buffer`.
    ///
    /// If the corresponding `Op` does not exist in the table yet, a new entry
    /// is inserted using the value returned by `<Op as Default>::default()`.
    crate fn get_index_for_resource(&mut self, cmd_buffer: &mut CmdBuffer, res: &Res) -> usize {
        let index = res.tracked_state().ref_table_index_mut(cmd_buffer);
        if let Some(index) = index.get() {
            index
        } else {
            let new_index = self.0.len();
            self.0.push((res.clone(), Op::default()));
            *index = Some(new_index).into();

            new_index
        }
    }

    /// Get a mutable reference to an entry by resource.
    crate fn get_mut(
        &mut self,
        cmd_buffer: &mut CmdBuffer,
        res: &Res,
    ) -> RefTableEntryMut<'_, Res, Op> {
        let index = self.get_index_for_resource(cmd_buffer, res);
        self.get_mut_by_index(index)
    }

    /// Get a mutable reference to an entry by index.
    crate fn get_mut_by_index(&mut self, index: usize) -> RefTableEntryMut<'_, Res, Op> {
        let ref mut e = self.0[index];
        RefTableEntryMut {
            index,
            resource: &e.0,
            op: &mut e.1,
        }
    }

    /// Get a reference to an entry by index.
    crate fn get_by_index(&self, index: usize) -> RefTableEntry<'_, Res, Op> {
        let ref e = self.0[index];
        RefTableEntry {
            index,
            resource: &e.0,
            op: &e.1,
        }
    }

    /// Clear the reference table. Additionally, transform the `latest`
    /// (last known state) of each referenced resource based on the recorded
    /// `Op`.
    ///
    /// For each referenced resource, `f` is called with the `latest` and `Op`
    /// of the resource.
    crate fn commit(
        &mut self,
        queue: &mut Queue,
        cmd_buffer: &mut CmdBuffer,
        mut f: impl FnMut(&Res, &mut Res::State, Op),
    ) {
        for (i, (res, op)) in self.0.drain(..).enumerate() {
            let tracked_state = res.tracked_state();

            let ref_table_index = tracked_state.ref_table_index_mut(cmd_buffer);
            debug_assert_eq!(ref_table_index.get(), Some(i));

            // The runtime check performed by `latest_mut` is superfluous
            // in practice, but can't omit it without making it unsound
            // (Unless, `Resource` is `unsafe trait`...)
            let latest = tracked_state.latest_mut(queue);
            f(&res, latest, op);

            // The resource is no longer referenced by this refernece table, so
            // clear the index
            *ref_table_index = None.into();
        }
    }

    /// Clear the reference table without modifying `latest`. `f` is called with
    /// each referenced resource.
    crate fn clear(&mut self, cmd_buffer: &mut CmdBuffer, mut f: impl FnMut(&Res, Op)) {
        for (i, (res, op)) in self.0.drain(..).enumerate() {
            let tracked_state = res.tracked_state();

            let ref_table_index = tracked_state.ref_table_index_mut(cmd_buffer);
            debug_assert_eq!(ref_table_index.get(), Some(i));

            f(&res, op);

            // The resource is no longer referenced by this refernece table, so
            // clear the index
            *ref_table_index = None.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[derive(Debug)]
    struct MyResource {
        tracked_state: TrackedState<MyResourceState>,
    }

    type MyResourceState = String;

    impl Resource for Rc<MyResource> {
        type State = MyResourceState;

        fn tracked_state(&self) -> &TrackedState<Self::State> {
            &self.tracked_state
        }
    }

    type MyResourceOp = String;

    #[test]
    fn test() {
        let (mut queue, mut cbs) = new_queue();
        let res = Rc::new(MyResource {
            tracked_state: TrackedState::new(queue.queue_id(), ":)".to_owned()),
        });

        debug_assert_eq!(res.tracked_state.latest_mut(&mut queue), ":)");

        // Create command buffers
        let mut cb1 = {
            let mut cb = cbs.pop().unwrap();
            let mut ref_table = RefTable::new();

            *ref_table.get_mut(&mut cb, &res).op += "[cb1]";

            (cb, ref_table)
        };

        let mut cb2 = {
            let mut cb = cbs.pop().unwrap();
            let mut ref_table = RefTable::new();

            *ref_table.get_mut(&mut cb, &res).op += "[cb2]";

            (cb, ref_table)
        };

        // Execute command buffers
        for (cb, ref_table) in &mut [cb2, cb1] {
            ref_table.commit(&mut queue, cb, |_res, latest, op: MyResourceOp| {
                *latest += "-";
                *latest += &op;
            })
        }

        // Validate the result
        debug_assert_eq!(res.tracked_state.latest_mut(&mut queue), ":)-[cb2]-[cb1]");
    }
}
