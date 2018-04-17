use libc::*;
use ::list::{ENetList, ENetListNode};
use ::address::ENetAddress;
use ::{ENetChannel, ENetAcknowledgement, ENetIncomingCommand, ENetOutgoingCommand};
use ::host::ENetHost;
use ::protocol::ENetProtocol;
use packet::ENetPacket;

pub const PEER_DEFAULT_ROUND_TRIP_TIME: size_t = 500;
pub const PEER_DEFAULT_PACKET_THROTTLE: size_t = 32;
pub const PEER_PACKET_THROTTLE_SCALE: size_t = 32;
pub const PEER_PACKET_THROTTLE_COUNTER: size_t = 7;
pub const PEER_PACKET_THROTTLE_ACCELERATION: size_t = 2;
pub const PEER_PACKET_THROTTLE_DECELERATION: size_t = 2;
pub const PEER_PACKET_THROTTLE_INTERVAL: size_t = 5000;
pub const PEER_PACKET_LOSS_SCALE: size_t = 1<<16;
pub const PEER_PACKET_LOSS_INTERVAL: size_t = 10000;
pub const PEER_WINDOW_SIZE_SCALE: size_t = 64 * 1024;
pub const PEER_TIMEOUT_LIMIT: size_t = 32;
pub const PEER_TIMEOUT_MINIMUM: size_t = 5000;
pub const PEER_TIMEOUT_MAXIMUM: size_t = 30000;
pub const PEER_PING_INTERVAL: size_t = 500;
pub const PEER_UNSEQUENCED_WINDOWS: size_t = 64;
pub const PEER_UNSEQUENCED_WINDOW_SIZE: size_t = 1024;
pub const PEER_FREE_UNSEQUENCED_WINDOWS: size_t = 32;
pub const PEER_RELIABLE_WINDOWS: size_t = 16;
pub const PEER_RELIABLE_WINDOW_SIZE: size_t = 0x1000;
pub const PEER_FREE_RELIABLE_WINDOWS: size_t = 8;

#[repr(C)]
#[derive(Debug)]
pub struct ENetPeer {
    pub dispatch_list: ENetListNode,
    pub host: *mut ENetHost,
    pub outgoing_peer_id: uint16_t,
    pub incoming_peer_id: uint16_t,
    pub connect_id: uint32_t,
    pub outgoing_session_id: uint8_t,
    pub incoming_session_id: uint8_t,
    pub address: ENetAddress,
    pub data: *mut c_void,
    pub state: ENetPeerState,
    pub channels: *mut ENetChannel,
    pub channel_count: size_t,
    pub incoming_bandwidth: uint32_t,
    pub outgoing_bandwidth: uint32_t,
    pub incoming_bandwidth_throttle_epoch: uint32_t,
    pub outgoing_bandwidth_throttle_epoch: uint32_t,
    pub incoming_data_total: uint32_t,
    pub outgoing_data_total: uint32_t,
    pub last_send_time: uint32_t,
    pub last_receive_time: uint32_t,
    pub next_timeout: uint32_t,
    pub earliest_timeout: uint32_t,
    pub packet_loss_epoch: uint32_t,
    pub packets_sent: uint32_t,
    pub packets_lost: uint32_t,
    pub packet_loss: uint32_t,
    pub packet_loss_variance: uint32_t,
    pub packet_throttle: uint32_t,
    pub packet_throttle_limit: uint32_t,
    pub packet_throttle_counter: uint32_t,
    pub packet_throttle_epoch: uint32_t,
    pub packet_throttle_acceleration: uint32_t,
    pub packet_throttle_deceleration: uint32_t,
    pub packet_throttle_interval: uint32_t,
    pub ping_interval: uint32_t,
    pub timeout_limit: uint32_t,
    pub timeout_maximum: uint32_t,
    pub timeout_minimum: uint32_t,
    pub last_round_trip_time: uint32_t,
    pub lowest_round_trip_time: uint32_t,
    pub last_round_trip_time_variance: uint32_t,
    pub highest_round_trip_time_variance: uint32_t,
    pub round_trip_time: uint32_t,
    pub round_trip_time_variance: uint32_t,
    pub mtu: uint32_t,
    pub window_size: uint32_t,
    pub reliabledata_in_transit: uint32_t,
    pub outgoing_reliable_sequence_number: uint16_t,
    pub acknowledgements: ENetList,
    pub sent_reliable_commands: ENetList,
    pub sent_unreliable_commands: ENetList,
    pub outgoing_reliable_commands: ENetList,
    pub outgoing_unreliable_commands: ENetList,
    pub dispatch_commands: ENetList,
    pub needs_dispatch: c_int,
    pub incoming_unsequenced_group: uint16_t,
    pub outgoing_unsequenced_group: uint16_t,
    pub unsequenced_window: [uint32_t; PEER_UNSEQUENCED_WINDOW_SIZE/32],
    pub event_data: uint32_t,
    pub total_waiting_data: size_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ENetPeerState {
    Disconnected = 0,
    Connecting = 1,
    AcknowledgingConnect = 2,
    ConnectionPending = 3,
    ConnectionSucceeded = 4,
    Connected = 5,
    DisconnectLater = 6,
    Disconnecting = 7,
    AcknowledgingDisconnect = 8,
    Zombie = 9,
}

extern {
    pub fn enet_peer_disconnect(peer: *mut ENetPeer, data: uint32_t);
    pub fn enet_peer_disconnect_later(peer: *mut ENetPeer, data: uint32_t);
    pub fn enet_peer_disconnect_now(peer: *mut ENetPeer, data: uint32_t);
    pub fn enet_peer_dispatch_incoming_reliable_commands(peer: *mut ENetPeer, channel: *mut ENetChannel);
    pub fn enet_peer_dispatch_incoming_unreliable_commands(peer: *mut ENetPeer, channel: *mut ENetChannel);
    pub fn enet_peer_on_connect(peer: *mut ENetPeer);
    pub fn enet_peer_on_disconnect(peer: *mut ENetPeer);
    pub fn enet_peer_ping(peer: *mut ENetPeer);
    pub fn enet_peer_ping_interval(peer: *mut ENetPeer, pingInterval: uint32_t);
    pub fn enet_peer_queue_acknowledgement(peer: *mut ENetPeer, command: *const ENetProtocol,
            sentTime: uint16_t) -> *mut ENetAcknowledgement;
    pub fn enet_peer_queue_incoming_command(peer: *mut ENetPeer, command: *const ENetProtocol,
            data: *const c_void, dataLength: size_t, flags: uint32_t, fragmentCount: uint32_t)
            -> *mut ENetIncomingCommand;
    pub fn enet_peer_queue_outgoing_command(peer: *mut ENetPeer, command: *const ENetProtocol,
            packet: *mut ENetPacket, offset: uint32_t, length: uint16_t) -> *mut ENetOutgoingCommand;
    pub fn enet_peer_receive(peer: *mut ENetPeer, channelID: *mut uint8_t) -> *mut ENetPacket;
    pub fn enet_peer_reset(peer: *mut ENetPeer);
    pub fn enet_peer_reset_queues(peer: *mut ENetPeer);
    pub fn enet_peer_send(peer: *mut ENetPeer, channelID: uint8_t, packet: *mut ENetPacket) -> c_int;
    pub fn enet_peer_setup_outgoing_command(peer: *mut ENetPeer,
            outgoingCommand: *mut ENetOutgoingCommand);
    pub fn enet_peer_throttle(peer: *mut ENetPeer, rtt: uint32_t) -> c_int;
    pub fn enet_peer_throttle_configure(peer: *mut ENetPeer, interval: uint32_t, acceleration: uint32_t,
            deceleration: uint32_t);
    pub fn enet_peer_timeout(peer: *mut ENetPeer, timeoutLimit: uint32_t, timeoutMinimum: uint32_t,
            timeoutMaximum: uint32_t);
}
