use libc::*;
use ENetBuffer;

pub type ENetPacketFreeCallback = extern "C" fn(packet: *mut ENetPacket);

#[repr(C)]
pub struct ENetPacket {
    pub referenceCount: size_t,
    pub flags: ENetPacketFlags,
    pub data: *mut uint8_t,
    pub dataLength: size_t,
    pub freeCallback: ENetPacketFreeCallback,
    pub userData: *mut c_void,
}

bitflags! {
    #[repr(C)]
    pub struct ENetPacketFlags: u32 {
        const RELIABLE = 1;
        const UNSEQUENCED = 0b10;
        const NO_ALLOCATE = 0b100;
        const UNRELIABLE_FRAGMENT = 0b1000;
        const SENT = 0b100000000;
    }
}

extern "C" {
    pub fn enet_crc32(buffers: *const ENetBuffer, bufferCount: size_t) -> uint32_t;
    pub fn enet_packet_create(
        data: *const c_void,
        dataLength: size_t,
        flags: ENetPacketFlags,
    ) -> *mut ENetPacket;
    pub fn enet_packet_destroy(packet: *mut ENetPacket);
    pub fn enet_packet_resize(packet: *mut ENetPacket, dataLength: size_t) -> c_int;
}
