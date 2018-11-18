//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
mod entry;

pub use self::entry::ngsengine_create;

// Register jemalloc as the global allocator.
// Disable this on `*-windows-msvc` since `jemallocator` doesn't build on it yet.
#[cfg(any(not(target_os = "windows"), target_env = "gnu"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
