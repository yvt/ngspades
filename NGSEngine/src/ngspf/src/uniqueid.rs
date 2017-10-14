//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::process::abort;
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::cell::Cell;

/// Process unique ID. Faster alternative to the `snowflake` crate that may use
/// 128-bit IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessUniqueId(u64);

static NEXT_GLOBAL_ID: AtomicUsize = ATOMIC_USIZE_INIT;

#[derive(Debug, Clone, Copy)]
struct ThreadState {
    global_id: u32,
    next_local_id: u32,
}

thread_local! {
    static STATE: Cell<ThreadState> = Cell::new(ThreadState {
        global_id: 0,
        next_local_id: 0,
    });
}

impl ProcessUniqueId {
    /// Construct a unique `ProcessUniqueId`.
    ///
    /// **Aborts** if it ran out of unique IDs.
    pub fn new() -> Self {
        if size_of::<usize>() < 8 {
            assert_eq!(size_of::<usize>(), 4);

            // Combine a thread ID and local ID
            STATE.with(|cell| {
                let mut th_state: ThreadState = cell.get();

                if th_state.global_id == 0 || th_state.next_local_id == <u32>::max_value() {
                    // Allocate a new thread ID
                    let gid = NEXT_GLOBAL_ID
                        .fetch_add(1, Ordering::Relaxed)
                        .wrapping_add(1);
                    if gid == 0 {
                        abort();
                    }
                    th_state.global_id = gid as u32;
                    th_state.next_local_id = 0;
                }

                let combined = (th_state.next_local_id as u64) | (th_state.global_id as u64) << 32;
                let ret = ProcessUniqueId(combined);

                th_state.next_local_id += 1;
                cell.set(th_state);

                ret
            })
        } else {
            // The global ID is already at least 64 bit wide
            let gid = NEXT_GLOBAL_ID
                .fetch_add(1, Ordering::Relaxed)
                .wrapping_add(1);
            if gid == 0 {
                abort();
            }
            ProcessUniqueId(gid as u64)
        }
    }
}

#[test]
fn test_uniqueness() {
    let id1 = ProcessUniqueId::new();
    let id2 = ProcessUniqueId::new();
    assert_ne!(id1, id2);
}
