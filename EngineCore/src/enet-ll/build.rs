extern crate cc;

fn main() {
    cc::Build::new()
        .file("libenet/callbacks.c")
        .file("libenet/compress.c")
        .file("libenet/host.c")
        .file("libenet/list.c")
        .file("libenet/packet.c")
        .file("libenet/peer.c")
        .file("libenet/protocol.c")
        .file("libenet/unix.c")
        .file("libenet/win32.c")
        .include("libenet/include")
        .compile("libenet.a");
}
