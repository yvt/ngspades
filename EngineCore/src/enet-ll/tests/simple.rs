extern crate enet_ll;

#[test]
fn initialize() {
    unsafe { enet_ll::enet_initialize() };
}