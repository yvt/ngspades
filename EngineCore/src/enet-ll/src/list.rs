#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetList {
    pub sentinel: ENetListNode,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetListNode {
    pub next: *mut ENetListNode,
    pub previous: *mut ENetListNode,
}
