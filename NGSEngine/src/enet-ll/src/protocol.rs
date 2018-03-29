use libc::*;

pub const PROTOCOL_MINIMUM_MTU: size_t = 576;
pub const PROTOCOL_MAXIMUM_MTU: size_t = 4096;
pub const PROTOCOL_MAXIMUM_PACKET_COMMANDS: size_t = 32;
pub const PROTOCOL_MINIMUM_WINDOW_SIZE: size_t = 4096;
pub const PROTOCOL_MAXIMUM_WINDOW_SIZE: size_t = 65536;
pub const PROTOCOL_MINIMUM_CHANNEL_COUNT: size_t = 1;
pub const PROTOCOL_MAXIMUM_CHANNEL_COUNT: size_t = 255;
pub const PROTOCOL_MAXIMUM_PEER_ID: size_t = 0xfff;
pub const PROTOCOL_MAXIMUM_FRAGMENT_COUNT: size_t = 1024 * 1024;

pub const PROTOCOL_COMMAND_NONE: uint8_t = 0;
pub const PROTOCOL_COMMAND_ACKNOWLEDGE: uint8_t = 1;
pub const PROTOCOL_COMMAND_CONNECT: uint8_t = 2;
pub const PROTOCOL_COMMAND_VERIFY_CONNECT: uint8_t = 3;
pub const PROTOCOL_COMMAND_DISCONNECT: uint8_t = 4;
pub const PROTOCOL_COMMAND_PING: uint8_t = 5;
pub const PROTOCOL_COMMAND_SEND_RELIABLE: uint8_t = 6;
pub const PROTOCOL_COMMAND_SEND_UNRELIABLE: uint8_t = 7;
pub const PROTOCOL_COMMAND_SEND_FRAGMENT: uint8_t = 8;
pub const PROTOCOL_COMMAND_SEND_UNSEQUENCED: uint8_t = 9;
pub const PROTOCOL_COMMAND_BANDWIDTH_LIMIT: uint8_t = 10;
pub const PROTOCOL_COMMAND_THROTTLE_CONFIGURE: uint8_t = 11;
pub const PROTOCOL_COMMAND_SEND_UNRELIABLE_FRAGMENT: uint8_t = 12;
pub const PROTOCOL_COMMAND_COUNT: uint8_t = 13;

pub const PROTOCOL_COMMAND_MASK: uint8_t = 0x0F;

#[repr(C)]
#[derive(Clone, Copy)]
pub union ENetProtocol {
    _size: [u8; 48],
    pub header: ENetProtocolCommandHeader,
    pub acknowledge: ENetProtocolAcknowledge,
    pub connect: ENetProtocolConnect,
    pub verify_connect: ENetProtocolVerifyConnect,
    pub disconnect: ENetProtocolDisconnect,
    pub ping: ENetProtocolPing,
    pub send_reliable: ENetProtocolSendReliable,
    pub send_unreliable: ENetProtocolSendUnreliable,
    pub send_unsequenced: ENetProtocolSendUnsequenced,
    pub send_fragment: ENetProtocolSendFragment,
    pub bandwidth_limit: ENetProtocolBandwidthLimit,
    pub throttle_configure: ENetProtocolThrottleConfigure,
}

use std::fmt;
impl fmt::Debug for ENetProtocol {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut fmt = fmt.debug_struct("ENetProtocol");
        unsafe {
            let fmt = fmt.field("header", &self.header);
            match self.header.command & PROTOCOL_COMMAND_MASK {
                PROTOCOL_COMMAND_ACKNOWLEDGE => {
                    fmt.field("acknowledge", &self.acknowledge).finish()
                }
                PROTOCOL_COMMAND_CONNECT => fmt.field("connect", &self.connect).finish(),
                PROTOCOL_COMMAND_VERIFY_CONNECT => {
                    fmt.field("verify_connect", &self.verify_connect).finish()
                }
                PROTOCOL_COMMAND_DISCONNECT => fmt.field("disconnect", &self.disconnect).finish(),
                PROTOCOL_COMMAND_PING => fmt.field("ping", &self.ping).finish(),
                PROTOCOL_COMMAND_SEND_RELIABLE => {
                    fmt.field("send_reliable", &self.send_reliable).finish()
                }
                PROTOCOL_COMMAND_SEND_UNRELIABLE => {
                    fmt.field("send_unreliable", &self.send_unreliable).finish()
                }
                PROTOCOL_COMMAND_SEND_FRAGMENT => {
                    fmt.field("send_fragment", &self.send_fragment).finish()
                }
                PROTOCOL_COMMAND_SEND_UNSEQUENCED => {
                    fmt.field("send_unsequenced", &self.send_unsequenced)
                        .finish()
                }
                PROTOCOL_COMMAND_BANDWIDTH_LIMIT => {
                    fmt.field("bandwidth_limit", &self.bandwidth_limit).finish()
                }
                PROTOCOL_COMMAND_THROTTLE_CONFIGURE => {
                    fmt.field("throttle_configure", &self.throttle_configure)
                        .finish()
                }
                PROTOCOL_COMMAND_SEND_UNRELIABLE_FRAGMENT => {
                    fmt.field("send_fragment", &self.send_fragment).finish()
                }
                _ => fmt.finish(),
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolAcknowledge {
    pub header: ENetProtocolCommandHeader,
    pub received_reliable_sequence_number: uint16_t,
    pub received_sent_time: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolBandwidthLimit {
    pub header: ENetProtocolCommandHeader,
    pub incoming_bandwidth: uint32_t,
    pub outgoing_bandwidth: uint32_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolConnect {
    pub header: ENetProtocolCommandHeader,
    pub outgoing_peer_id: uint16_t,
    pub incoming_session_id: uint8_t,
    pub outgoing_session_id: uint8_t,
    pub mtu: uint32_t,
    pub window_size: uint32_t,
    pub channel_count: uint32_t,
    pub incoming_bandwidth: uint32_t,
    pub outgoing_bandwidth: uint32_t,
    pub packet_throttle_interval: uint32_t,
    pub packet_throttle_acceleration: uint32_t,
    pub packet_throttle_deceleration: uint32_t,
    pub connect_id: uint32_t,
    pub data: uint32_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolDisconnect {
    pub header: ENetProtocolCommandHeader,
    pub data: uint32_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolCommandHeader {
    pub command: uint8_t,
    pub channel_id: uint8_t,
    pub reliable_sequence_number: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolHeader {
    pub peer_id: uint16_t,
    pub send_time: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolPing {
    pub header: ENetProtocolCommandHeader,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolSendFragment {
    pub header: ENetProtocolCommandHeader,
    pub start_sequence_number: uint16_t,
    pub data_length: uint16_t,
    pub fragment_count: uint32_t,
    pub fragment_number: uint32_t,
    pub total_length: uint32_t,
    pub fragment_offset: uint32_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolSendReliable {
    pub header: ENetProtocolCommandHeader,
    pub data_length: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolSendUnreliable {
    pub header: ENetProtocolCommandHeader,
    pub unreliable_sequence_number: uint16_t,
    pub data_length: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolSendUnsequenced {
    pub header: ENetProtocolCommandHeader,
    pub unsequenced_group: uint16_t,
    pub data_length: uint16_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolThrottleConfigure {
    pub header: ENetProtocolCommandHeader,
    pub packet_throttle_interval: uint32_t,
    pub packet_throttle_acceleration: uint32_t,
    pub packet_throttle_deceleration: uint32_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetProtocolVerifyConnect {
    pub header: ENetProtocolCommandHeader,
    pub outgoing_peer_id: uint16_t,
    pub incoming_session_id: uint8_t,
    pub outgoing_session_id: uint8_t,
    pub mtu: uint32_t,
    pub window_size: uint32_t,
    pub channel_count: uint32_t,
    pub incoming_bandwidth: uint32_t,
    pub outgoing_bandwidth: uint32_t,
    pub packet_throttle_interval: uint32_t,
    pub packet_throttle_acceleration: uint32_t,
    pub packet_throttle_deceleration: uint32_t,
    pub connect_id: uint32_t,
}
