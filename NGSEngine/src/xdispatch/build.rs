extern crate cc;

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    let target_parts: Vec<_> = target.split('-').collect();
    let has_native_dispatch = target.ends_with("-apple-darwin");

    if !has_native_dispatch {
        // TODO: check if this works on other platforms
        let mut build = cc::Build::new();
        if target_parts[2] == "windows" {
            build
                .define("LIBPTHREAD_WORKQUEUE_EXPORTS", None)
                .define("MAKE_STATIC", None)
                .define("_USRDLL", None)
                .define("_WINDLL", None)
                .file("lib/libpthread_workqueue/windows/manager.c")
                .file("lib/libpthread_workqueue/windows/platform.c")
                .file("lib/libpthread_workqueue/windows/thread_info.c")
                .file("lib/libpthread_workqueue/windows/thread_rt.c");
        } else {
            build
                .file("lib/libpthread_workqueue/posix/manager.c")
                .file("lib/libpthread_workqueue/posix/thread_info.c")
                .file("lib/libpthread_workqueue/posix/thread_rt.c")
                .file("lib/libpthread_workqueue/api.c")
                .file("lib/libpthread_workqueue/witem_cache.c");
        }
        build
            .file("lib/libpthread_workqueue/api.c")
            .file("lib/libpthread_workqueue/witem_cache.c")
            .include("lib/include")
            .include("lib/libpthread_workqueue")
            .compile("libpthread_workqueue.a");

        let mut build = cc::Build::new();
        if target_parts[2] == "windows" {
            build
                .file("lib/libkqueue/windows/platform.c")
                .file("lib/libkqueue/windows/read.c")
                .file("lib/libkqueue/windows/timer.c")
                .file("lib/libkqueue/windows/user.c");
        } else {
            build
                .file("lib/libkqueue/linux/platform.c")
                .file("lib/libkqueue/linux/proc.c")
                .file("lib/libkqueue/linux/read.c")
                .file("lib/libkqueue/linux/signal.c")
                .file("lib/libkqueue/linux/timer.c")
                .file("lib/libkqueue/linux/user.c")
                .file("lib/libkqueue/linux/vnode.c")
                .file("lib/libkqueue/linux/write.c");
        }
        build
            .file("lib/libkqueue/common/filter.c")
            .file("lib/libkqueue/common/kevent.c")
            .file("lib/libkqueue/common/knote.c")
            .file("lib/libkqueue/common/kqueue.c")
            .file("lib/libkqueue/common/map.c")
            .include("lib/include")
            .compile("libkqueue.a");

        let mut build = cc::Build::new();
        if target_parts[2] == "windows" {
            build
                .file("lib/xdispatch/platform/windows/platform.c")
                .include("lib/xdispatch/platform/windows");
        }
        build
            .file("lib/xdispatch/src/apply.c")
            .file("lib/xdispatch/src/benchmark.c")
            .file("lib/xdispatch/src/blocks.c")
            .file("lib/xdispatch/src/continuation_cache.c")
            .file("lib/xdispatch/src/debug.c")
            .file("lib/xdispatch/src/legacy.c")
            .file("lib/xdispatch/src/object.c")
            .file("lib/xdispatch/src/once.c")
            .file("lib/xdispatch/src/protocolServer.c")
            .file("lib/xdispatch/src/protocolUser.c")
            .file("lib/xdispatch/src/queue.c")
            .file("lib/xdispatch/src/queue_kevent.c")
            .file("lib/xdispatch/src/semaphore.c")
            .file("lib/xdispatch/src/shared_constructor.c")
            .file("lib/xdispatch/src/source.c")
            .file("lib/xdispatch/src/source_kevent.c")
            .file("lib/xdispatch/src/time.c")
            .file("lib/xdispatch/src/shims/time.c")
            .file("lib/xdispatch/src/shims/tsd.c")
            .include("lib/include")
            .compile("libxdispatch.a");
    }
}
