//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
mod entry;

pub use self::entry::ngsengine_create;

// Register jemalloc as the global allocator
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
