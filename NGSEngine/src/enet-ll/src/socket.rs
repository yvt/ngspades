use libc::*;
use address::ENetAddress;
use ::ENetBuffer;

pub type ENetSocket = c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ENetSocketType {
    Stream = 1,
    Datagram = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ENetSocketOption {
    NonBlock = 1,
    Broadcast = 2,
    ReceiveBuffer = 3,
    SendBuffer = 4,
    ReuseAddress = 5,
    ReceiveTimeOut = 6,
    SendTimeOut = 7,
    Error = 8,
    NoDelay = 9,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ENetSocketShutdown {
    Read = 0,
    Write = 1,
    ReadWrite = 2,
}

extern {
    pub fn enet_socket_accept(socket: ENetSocket, address: *mut ENetAddress) -> ENetSocket;
    pub fn enet_socket_bind(socket: ENetSocket, address: *const ENetAddress) -> c_int;
    pub fn enet_socket_connect(socket: ENetSocket, address: *const ENetAddress) -> c_int;
    pub fn enet_socket_create(socketType: ENetSocketType) -> ENetSocket;
    pub fn enet_socket_destroy(socket: ENetSocket);
    pub fn enet_socket_get_address(socket: ENetSocket, address: *mut ENetAddress) -> c_int;
    pub fn enet_socket_get_option(socket: ENetSocket, socketOption: ENetSocketOption,
            _unknown: *mut c_int) -> c_int;
    pub fn enet_socket_listen(socket: ENetSocket, _unknown: c_int) -> c_int;
    pub fn enet_socket_receive(socket: ENetSocket, address: *mut ENetAddress, buffer: *mut ENetBuffer,
            bufferSize: size_t) -> c_int;
    pub fn enet_socket_send(socket: ENetSocket, address: *const ENetAddress, buffer: *const ENetBuffer,
            bufferSize: size_t) -> c_int;
    pub fn enet_socket_set_option(socket: ENetSocket, socketOption: ENetSocketOption, _unknown: c_int)
            -> c_int;
    pub fn enet_socket_shutdown(socket: ENetSocket, socketShutdown: ENetSocketShutdown) -> c_int;
    pub fn enet_socket_wait(socket: ENetSocket, _unknownA: *mut uint32_t, _unknownB: uint32_t) -> c_int;
}
