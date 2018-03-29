//! ENet and Low-Level Interfaces
//! =============================
//!
//! Almost everything in `src/` was taken from [enet-sys](https://github.com/ruabmbua/enet-sys)
//! which is licensed under the MIT license:
//!
//! > The MIT License (MIT)
//! >
//! > Copyright (c) 2016 Roland Ruckerbauer
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a copy
//! > of this software and associated documentation files (the "Software"), to deal
//! > in the Software without restriction, including without limitation the rights
//! > to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! > copies of the Software, and to permit persons to whom the Software is
//! > furnished to do so, subject to the following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included in all
//! > copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! > IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! > FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! > LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! > SOFTWARE.
//!
//! [ENet](http://enet.bespin.org) itself (`libenet/`) is also licensed under the MIT license:
//!
//! > The MIT License (MIT)
//! >
//! > Copyright (c) 2002-2016 Lee Salzman
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a copy
//! > of this software and associated documentation files (the "Software"), to deal
//! > in the Software without restriction, including without limitation the rights
//! > to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! > copies of the Software, and to permit persons to whom the Software is
//! > furnished to do so, subject to the following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included in all
//! > copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! > IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! > FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! > LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! > SOFTWARE.

#![allow(non_snake_case)]

extern crate libc;

extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;

pub mod address;
pub mod host;
pub mod protocol;
pub mod list;
pub mod packet;
pub mod peer;
pub mod socket;

use libc::*;
use host::ENetHost;
use packet::ENetPacket;
use list::{ENetList, ENetListNode};
use peer::{PEER_RELIABLE_WINDOWS, ENetPeer};
use protocol::ENetProtocol;

pub type ENetVersion = uint32_t;
pub type ENetChecksumCallback = extern fn(buffers: *const ENetBuffer, bufferCount: size_t)
        -> uint32_t;
pub type ENetInterceptCallback = extern fn(host: *mut ENetHost, event: *mut ENetEvent);

pub const ENET_HOST_ANY : uint32_t = 0;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetCallbacks {
    pub free: extern fn(memory: *mut c_void),
    pub malloc: extern fn(size: size_t) -> *mut c_void,
    pub no_memory: extern fn(),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetBuffer {
    pub data: *mut c_void,
    pub data_length: size_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetCompressor {
    pub context: *mut c_void,
    pub compress: extern fn(context: *mut c_void, inBuffers: *const ENetBuffer,
            inBufferCount: size_t, inLimit: size_t, outData: *mut uint8_t, outLimit: size_t)
            -> size_t,
    pub decompress: extern fn(context: *mut c_void, inData: *const uint8_t, inLimit: size_t,
            outData: *mut uint8_t, outLimit: size_t) -> size_t,
    pub destroy: extern fn(context: *mut c_void),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetEvent {
    pub _type: ENetEventType,
    pub peer: *mut ENetPeer,
    pub channel_id: uint8_t,
    pub data: uint32_t,
    pub packet: *mut ENetPacket,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ENetEventType {
    None = 0,
    Connect = 1,
    Disconnect = 2,
    Receive = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ENetChannel {
    pub outgoing_reliable_sequence_number: uint16_t,
    pub outgoing_unrelianle_sequence_number: uint16_t,
    pub used_reliable_windows: uint16_t,
    pub reliable_windows: [uint16_t; PEER_RELIABLE_WINDOWS],
    pub incoming_reliable_sequence_number: uint16_t,
    pub incoming_unreliable_sequence_number: uint16_t,
    pub incoming_reliable_commands: ENetList,
    pub incoming_unreliable_commands: ENetList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ENetAcknowledgement {
    pub achnowledgement_list: ENetListNode,
    pub sent_time: uint32_t,
    pub command: ENetProtocol,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ENetIncomingCommand {
    pub incoming_commands_list: ENetListNode,
    pub reliable_sequence_number: uint16_t,
    pub unreliable_sequence_number: uint16_t,
    pub command: ENetProtocol,
    pub fragment_count: uint32_t,
    pub fragments_remaining: uint32_t,
    pub fragments: *mut uint32_t,
    pub packet: *mut ENetPacket,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ENetOutgoingCommand {
    pub outgoing_command_list: ENetListNode,
    pub reliable_sequence_number: uint16_t,
    pub unreliable_sequence_number: uint16_t,
    pub sent_time: uint32_t,
    pub round_trip_timeout: uint32_t,
    pub round_trip_timeout_limit: uint32_t,
    pub fragment_offset: uint32_t,
    pub fragment_length: uint16_t,
    pub send_attempts: uint16_t,
    pub command: ENetProtocol,
    pub packet: *mut ENetPacket,
}

extern {
    pub fn enet_deinitialize();
    pub fn enet_initialize() -> c_int;
    pub fn enet_initialize_with_callbacks(version: ENetVersion, ) -> c_int;
    pub fn enet_linked_version() -> ENetVersion;
}
