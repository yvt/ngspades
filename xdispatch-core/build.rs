extern crate cc;

use std::env;

const SUPPORTED_TARGETS: &[&str] = &[
    "-darwin",
    "-linux-gnu",
    "-windows-msvc",
    "-windows-gcc",
];

fn main() {
    let target = env::var("TARGET").unwrap();
    let target_parts: Vec<_> = target.split('-').collect();

    if !SUPPORTED_TARGETS.iter().any(|t| target.ends_with(t)) {
        panic!("xdispatch-core does not support target: {}", target);
    }

    let has_native_dispatch = target.ends_with("-apple-darwin");

    if has_native_dispatch {
        return;
    }

    // TODO: check if this works on Windows
    // TODO: check if this works on Linux

    let mut build = cc::Build::new();
    if target_parts[2] == "windows" {
        build
            .define("LIBPTHREAD_WORKQUEUE_EXPORTS", None)
            .define("WIN_PTHREAD_EXPORT", Some(""))
            .define("MAKE_STATIC", None)
            .define("_USRDLL", None)
            .define("_WINDLL", None)
            .file("xdispatch/libpthread_workqueue/src/windows/manager.c")
            .file("xdispatch/libpthread_workqueue/src/windows/platform.c")
            .file("xdispatch/libpthread_workqueue/src/windows/thread_info.c")
            .file("xdispatch/libpthread_workqueue/src/windows/thread_rt.c");
        if target_parts[0] == "x86_64" {
            build.define("__x86_64__", None);
        } else if target_parts[0] == "i386" {
            build.define("__i386__", None);
        }
    } else {
        build
            .file("xdispatch/libpthread_workqueue/src/posix/manager.c")
            .file("xdispatch/libpthread_workqueue/src/posix/thread_info.c")
            .file("xdispatch/libpthread_workqueue/src/posix/thread_rt.c");
    }
    build
        .file("xdispatch/libpthread_workqueue/src/api.c")
        .file("xdispatch/libpthread_workqueue/src/witem_cache.c")
        .include("xdispatch/libpthread_workqueue/include")
        .include("xdispatch/libpthread_workqueue/src")
        .compile("libpthread_workqueue.a");

    let mut build = cc::Build::new();
    if target_parts[2] == "windows" {
        build
            .define("MAKE_STATIC", None)
            .file("xdispatch/libkqueue/src/windows/platform.c")
            .file("xdispatch/libkqueue/src/windows/read.c")
            .file("xdispatch/libkqueue/src/windows/timer.c")
            .file("xdispatch/libkqueue/src/windows/user.c");
    } else {
        build
            .file("xdispatch/libkqueue/src/linux/platform.c")
            .file("xdispatch/libkqueue/src/linux/read.c")
            .file("xdispatch/libkqueue/src/linux/signal.c")
            .file("xdispatch/libkqueue/src/linux/timer.c")
            .file("xdispatch/libkqueue/src/linux/user.c")
            .file("xdispatch/libkqueue/src/linux/vnode.c")
            .file("xdispatch/libkqueue/src/linux/write.c");
    }
    build
        .file("xdispatch/libkqueue/src/common/filter.c")
        .file("xdispatch/libkqueue/src/common/kevent.c")
        .file("xdispatch/libkqueue/src/common/knote.c")
        .file("xdispatch/libkqueue/src/common/kqueue.c")
        .file("xdispatch/libkqueue/src/common/map.c")
        .include("xdispatch/libkqueue/include")
        .include("xdispatch/libkqueue/src/common")
        .compile("libkqueue.a");

    let mut build = cc::Build::new();
    if target_parts[2] == "windows" {
        build
            .define("WIN_PTHREAD_EXPORT", Some(""))
            .file("xdispatch/core/platform/windows/platform.c")
            .include("xdispatch/core/platform/windows");
    } else {
        build.include("xdispatch/core/platform/posix");
    }
    build
        .file("xdispatch/core/src/apply.c")
        .file("xdispatch/core/src/benchmark.c")
        .file("xdispatch/core/src/blocks.c")
        .file("xdispatch/core/src/continuation_cache.c")
        .file("xdispatch/core/src/debug.c")
        .file("xdispatch/core/src/legacy.c")
        .file("xdispatch/core/src/object.c")
        .file("xdispatch/core/src/once.c")
        .file("xdispatch/core/src/protocolServer.c")
        .file("xdispatch/core/src/protocolUser.c")
        .file("xdispatch/core/src/queue.c")
        .file("xdispatch/core/src/queue_kevent.c")
        .file("xdispatch/core/src/semaphore.c")
        .file("xdispatch/core/src/shared_constructor.c")
        .file("xdispatch/core/src/source.c")
        .file("xdispatch/core/src/source_kevent.c")
        .file("xdispatch/core/src/time.c")
        .file("xdispatch/core/src/shims/time.c")
        .file("xdispatch/core/src/shims/tsd.c")
        .include("xdispatch/libpthread_workqueue/include")
        .include("xdispatch")
        .include("xdispatch/core/include")
        .compile("libdispatch.a");
}
