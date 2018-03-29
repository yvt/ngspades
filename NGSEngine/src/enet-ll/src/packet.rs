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

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
pub enum ENetPacketFlag {
    Reliable = 1,
    Unsequenced = 0b10,
    NoAllocate = 0b100,
    UnreliableFragment = 0b1000,
    Sent = 0b100000000,
}

pub type ENetPacketFlags = ::ngsenumflags::BitFlags<ENetPacketFlag>;

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
