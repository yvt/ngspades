use libc::*;
use ::address::ENetAddress;
use ::{ENetBuffer, ENetChecksumCallback, ENetCompressor, ENetInterceptCallback, ENetEvent};
use ::protocol::{PROTOCOL_MAXIMUM_PACKET_COMMANDS, ENetProtocol, PROTOCOL_MAXIMUM_MTU};
use ::list::ENetList;
use ::peer::ENetPeer;
use ::socket::ENetSocket;
use ::packet::ENetPacket;

pub const HOST_RECEIVE_BUFFER_SIZE: size_t = 256 * 1024;
pub const HOST_SEND_BUFFER_SIZE: size_t = 256 * 1024;
pub const HOST_BANDWIDTH_THROTTLE_INTERVAL: size_t = 1000;
pub const HOST_DEFAULT_MTU: size_t = 1400;
pub const HOST_DEFAULT_MAXIMUM_PACKET_SIZE: size_t = 32 * 1024 * 1024;
pub const HOST_DEFAULT_MAXIMUM_WAITING_DATA: size_t = 32 * 1024 * 1024;

#[repr(C)]
pub struct ENetHost {
    pub socket: ENetSocket,
    pub address: ENetAddress,
    pub incoming_bandwidth: uint32_t,
    pub outgoing_bandwidth: uint32_t,
    pub bandwidth_throttle_epoch: uint32_t,
    pub mtu: uint32_t,
    pub random_seed: uint32_t,
    pub recalculate_bandwidth_limits: c_int,
    pub peers: *mut ENetPeer,
    pub peer_count: size_t,
    pub channel_limit: size_t,
    pub service_time: uint32_t,
    pub dispatch_queue: ENetList,
    pub continue_sending: c_int,
    pub packet_size: size_t,
    pub header_flags: uint16_t,
    pub commands: [ENetProtocol; PROTOCOL_MAXIMUM_PACKET_COMMANDS],
    pub command_count: size_t,
    pub buffers: [ENetBuffer; 1+2 * PROTOCOL_MAXIMUM_PACKET_COMMANDS],
    pub buffer_count: size_t,
    pub checksum: ENetChecksumCallback,
    pub compressor: ENetCompressor,
    pub packet_data: [[uint8_t; PROTOCOL_MAXIMUM_MTU]; 2],
    pub received_address: ENetAddress,
    pub received_data: *mut uint8_t,
    pub received_data_length: size_t,
    pub total_sent_data: uint32_t,
    pub total_sent_packets: uint32_t,
    pub total_received_data: uint32_t,
    pub total_received_packets: uint32_t,
    pub intercept: ENetInterceptCallback,
    pub connected_peers: size_t,
    pub bandwidth_limited_peers: size_t,
    pub duplicate_peers: size_t,
    pub maximum_packet_size: size_t,
    pub maximum_waiting_data: size_t,
}

extern {
    pub fn enet_host_bandwidth_limit(host: *mut ENetHost, incomingBandwidth: uint32_t,
            outgoingBandwidth: uint32_t);
    pub fn enet_host_bandwidth_throttle(host: *mut ENetHost);
    pub fn enet_host_broadcast(host: *mut ENetHost, channelID: uint8_t, packet: *mut ENetPacket);
    pub fn enet_host_channel_limit(host: *mut ENetHost, channelLimit: size_t);
    pub fn enet_host_check_events(host: *mut ENetHost, event: *mut ENetEvent) -> c_int;
    pub fn enet_host_compress(host: *mut ENetHost, compressor: *const ENetCompressor);
    pub fn enet_host_compress_with_range_coder(host: *mut ENetHost) -> c_int;
    pub fn enet_host_connect(host: *mut ENetHost, address: *const ENetAddress, channelCount: size_t,
            data: uint32_t) -> *mut ENetPeer;
    pub fn enet_host_create(address: *const ENetAddress, peerCount: size_t, channelLimit: size_t,
            incomingBandwidth: uint32_t, outgoingBandwidth: uint32_t) -> *mut ENetHost;
    pub fn enet_host_destroy(host: *mut ENetHost);
    pub fn enet_host_flush(host: *mut ENetHost);
    pub fn enet_host_service(host: *mut ENetHost, event: *mut ENetEvent, timeout: uint32_t) -> c_int;
}
