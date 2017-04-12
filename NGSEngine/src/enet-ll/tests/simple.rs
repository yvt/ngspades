extern crate enet;

#[test]
fn initialize() {
    unsafe { enet::enet_initialize() };
}