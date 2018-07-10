extern crate cc;

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    let target_parts: Vec<_> = target.split('-').collect();

    let mut build = cc::Build::new();

    build
        .file("libenet/callbacks.c")
        .file("libenet/compress.c")
        .file("libenet/host.c")
        .file("libenet/list.c")
        .file("libenet/packet.c")
        .file("libenet/peer.c")
        .file("libenet/protocol.c")
        .file("libenet/unix.c")
        .file("libenet/win32.c")
        .include("libenet/include");

    if target_parts[2] == "linux" {
        build.define("HAS_SOCKLEN_T", "1");
    }

    build.compile("libenet.a");
}
