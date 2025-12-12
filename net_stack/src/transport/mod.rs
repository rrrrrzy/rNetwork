// Copyright (C) 2025 rrrrrzy
// SPDX-License-Identifier: GPL-3.0-or-later
//
// --------------------------------------------------
// 致敬所有在深夜调试代码的灵魂。
// 即便 Bug 如山，我亦往矣。
// --------------------------------------------------
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use protocol::ipv4::Ipv4Addr;
use std::collections::HashMap;

use crate::transport::udp::UdpSocketState;

pub mod udp;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct SocketHandle {
    pub protocol: u8, // 6 -> TCP, 17 -> UDP
    pub local_addr: Ipv4Addr,
    pub local_port: u16,
    pub remote_addr: Ipv4Addr,
    pub remote_port: u16,
}

impl SocketHandle {
    pub fn new(
        protocol: &SocketType,
        local_addr: Ipv4Addr,
        local_port: u16,
        remote_addr: Ipv4Addr,
        remote_port: u16,
    ) -> Self {
        Self {
            protocol: protocol.to_code(),
            local_addr,
            local_port,
            remote_addr,
            remote_port,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Udp,
    Tcp,
    Unknown,
}

impl SocketType {
    pub fn to_code(&self) -> u8 {
        match self {
            Self::Tcp => 6,
            Self::Udp => 17,
            Self::Unknown => 0,
        }
    }

    pub fn parse(st: u8) -> Self {
        match st {
            6 => Self::Tcp,
            17 => Self::Udp,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum Socket {
    Udp(UdpSocketState),
    // Future: Tcp(TcpSocket)
}

pub struct SocketSet {
    sockets: HashMap<SocketHandle, Socket>,
}

impl SocketSet {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
        }
    }

    pub fn add(&mut self, handle: SocketHandle, socket: Socket) {
        self.sockets.insert(handle, socket);
    }

    pub fn get(&self, handle: SocketHandle) -> Option<&Socket> {
        self.sockets.get(&handle)
    }

    pub fn get_mut(&mut self, handle: SocketHandle) -> Option<&mut Socket> {
        self.sockets.get_mut(&handle)
    }

    pub fn remove(&mut self, handle: SocketHandle) -> Option<Socket> {
        self.sockets.remove(&handle)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&SocketHandle, &mut Socket)> {
        self.sockets.iter_mut()
    }

    /// 查找单播 Socket (Unicast Lookup)
    /// 返回最佳匹配的一个 Socket
    pub fn lookup(
        &mut self,
        protocol: &SocketType,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
    ) -> Option<&mut Socket> {
        // 1. 精确匹配 (5元组完整匹配)
        let socket_handle_exact = SocketHandle::new(
            protocol, dst_ip,   // 本地 IP
            dst_port, // 本地端口
            src_ip,   // 远程 IP
            src_port, // 远程端口
        );
        if self.sockets.contains_key(&socket_handle_exact) {
            return self.sockets.get_mut(&socket_handle_exact);
        }

        // 2. 监听特定 IP (Local IP 匹配, Remote 为 0)
        let socket_handle_specified = SocketHandle::new(
            protocol,
            dst_ip,
            dst_port,
            Ipv4Addr::unspecified(), // 0.0.0.0
            0,                       // 0
        );
        if self.sockets.contains_key(&socket_handle_specified) {
            return self.sockets.get_mut(&socket_handle_specified);
        }

        // 3. 监听所有 IP (Local IP 为 0, Remote 为 0)
        let socket_handle_wildcard = SocketHandle::new(
            protocol,
            Ipv4Addr::unspecified(), // 0.0.0.0
            dst_port,                // specified ports
            Ipv4Addr::unspecified(), // 0.0.0.0
            0,                       // 0
        );
        if self.sockets.contains_key(&socket_handle_wildcard) {
            return self.sockets.get_mut(&socket_handle_wildcard);
        }
        None
    }

    /// 查找多播/广播 Socket (Multicast/Broadcast Lookup)
    /// 返回所有匹配的 Socket 列表
    pub fn lookup_multicast(
        &mut self,
        protocol: &SocketType,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
    ) -> Vec<&mut Socket> {
        let any_ip = Ipv4Addr::unspecified();

        self.sockets
            .iter_mut()
            .filter_map(|(handle, socket)| {
                // 1. 协议匹配
                if handle.protocol != protocol.to_code() {
                    return None;
                }

                // 2. 本地端口必须匹配目的端口
                if handle.local_port != dst_port {
                    return None;
                }

                // 3. 本地 IP 匹配 (目的 IP)
                // Socket 必须绑定到该多播组 IP，或者绑定到 0.0.0.0 (INADDR_ANY)
                if handle.local_addr != dst_ip && handle.local_addr != any_ip {
                    return None;
                }

                // 4. 远程 IP 匹配 (源 IP)
                // 如果 Socket 指定了远程 IP (已连接)，则必须匹配源 IP
                // 否则 (远程 IP 为 0.0.0.0)，接受任何源 IP
                if handle.remote_addr != any_ip && handle.remote_addr != src_ip {
                    return None;
                }

                // 5. 远程端口匹配 (源端口)
                // 同上，如果指定了远程端口，必须匹配
                if handle.remote_port != 0 && handle.remote_port != src_port {
                    return None;
                }

                Some(socket)
            })
            .collect()
    }
}
