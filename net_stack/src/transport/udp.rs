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

use crate::{
    handlers,
    stack::NetworkStack,
    transport::{Socket, SocketHandle},
};
use anyhow::{self, Error};
use protocol::ipv4::Ipv4Addr;
use std::{collections::VecDeque, f32::consts::E, sync::Arc};

#[derive(Debug)]
pub struct UdpSocketState {
    /// Received packets queue: (source_ip, source_port, payload)
    /// UDP preserves message boundaries, so we store packets, not a byte stream.
    rx_queue: VecDeque<(Ipv4Addr, u16, Vec<u8>)>,

    /// Maximum number of packets to buffer in the receive queue
    rx_capacity: usize,

    /// To send packets queue: (Ipv4Addr, u16, Vec<u8>)
    /// UDP send messages unordered, so we store packets,
    /// waiting for Stack scheduling to send.
    tx_queue: VecDeque<(Ipv4Addr, u16, Vec<u8>)>,

    /// Maximun number of packets to buffer in the send queue
    tx_capacity: usize,
}

impl UdpSocketState {
    /// Create a new UDP socket
    pub fn new() -> Self {
        Self {
            rx_queue: VecDeque::new(),
            rx_capacity: 32, // Default buffer size
            tx_queue: VecDeque::new(),
            tx_capacity: 32, // Default buffer size
        }
    }

    /// Set the receive buffer capacity
    pub fn set_rx_capacity(&mut self, capacity: usize) {
        self.rx_capacity = capacity;
    }

    /// Set the send buffer capacity
    pub fn set_tx_capacity(&mut self, capacity: usize) {
        self.tx_capacity = capacity;
    }

    /// Push a received packet into the socket's buffer
    /// This is called by the network stack when a packet matches this socket.
    pub fn rx_enqueue(&mut self, src_ip: Ipv4Addr, src_port: u16, payload: &[u8]) {
        if self.rx_queue.len() < self.rx_capacity {
            self.rx_queue
                .push_back((src_ip, src_port, payload.to_vec()));
        } else {
            // Drop packet if buffer is full
            // In a real system we might want to log this or increment a counter
            // eprintln!("UDP Socket buffer full, dropping packet from {}:{}", src_ip, src_port);
        }
    }

    /// Pop a packet from the receive queue
    /// Returns (source_ip, source_port, payload)
    pub fn recv(&mut self) -> Option<(Ipv4Addr, u16, Vec<u8>)> {
        self.rx_queue.pop_front()
    }

    /// Check if there is data to read
    pub fn can_recv(&self) -> bool {
        !self.rx_queue.is_empty()
    }

    pub fn send_to(&mut self, payload: &[u8], dst_ip: Ipv4Addr, dst_port: u16) {
        if self.tx_queue.len() < self.tx_capacity {
            self.tx_queue
                .push_back((dst_ip, dst_port, payload.to_vec()));
        }
    }

    pub fn poll_transmit(&mut self) -> Option<(Ipv4Addr, u16, Vec<u8>)> {
        self.tx_queue.pop_front()
    }

    // pub fn send(
    //     &self,
    //     dst_mac: MacAddr,
    //     dst_ip: Ipv4Addr,
    //     src_port: u16,
    //     dst_port: u16,
    //     payload: &[u8],
    // ) {
    //     let src_ip = stack.config().ip;

    //     // 在这里构造 UdpPacket
    //     let header = protocol::udp::UdpHeader::new(src_port, dst_port, 0);
    //     let udp_packet = protocol::udp::UdpPacket::new(header, payload.to_vec(), src_ip, dst_ip);

    //     let udp_bytes = udp_packet.to_bytes();

    //     handlers::ipv4::send_packet(
    //         stack,
    //         dst_mac,
    //         dst_ip,
    //         protocol::ipv4::Ipv4Protocol::UDP,
    //         &udp_bytes,
    //     );
    // }
}

impl Default for UdpSocketState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UdpSocket {
    handle: SocketHandle,
    stack: Arc<NetworkStack>,
}

impl UdpSocket {
    pub fn bind(stack: Arc<NetworkStack>, addr: &str) -> anyhow::Result<Self> {
        let (ip, port) = parse_addr(addr)?;

        let handle = SocketHandle::new(
            &super::SocketType::Udp,
            ip,
            port,
            Ipv4Addr::unspecified(),
            0,
        );

        let socket_state = UdpSocketState::new();

        stack
            .sockets
            .lock()
            .unwrap()
            .add(handle, Socket::Udp(socket_state));

        Ok(Self { handle, stack })
    }

    pub fn send_to(&self, payload: &[u8], dst_addr: &str) -> anyhow::Result<()> {
        let (dst_ip, dst_port) = parse_addr(dst_addr)?;

        // 我们需要通过 handle 找到自己的 SocketState
        // 注意：lookup 是用来查找"匹配数据包的 Socket"，而这里我们需要"获取自己的 Socket"
        // 所以应该直接用 self.handle 去 get_mut

        let mut sockets = self.stack.sockets.lock().unwrap();
        if let Some(Socket::Udp(udp_socket_state)) = sockets.get_mut(self.handle) {
            udp_socket_state.send_to(payload, dst_ip, dst_port);
            Ok(())
        } else {
            // 这种情况理论上不应该发生，除非 Socket 被意外移除了
            anyhow::bail!("Socket state not found (maybe closed?)");
        }
    }

    pub fn recv_from(&self) -> anyhow::Result<(Vec<u8>, String)> {
        let mut sockets = self.stack.sockets.lock().unwrap();
        if let Some(Socket::Udp(udp_socket_state)) = sockets.get_mut(self.handle) {
            if let Some((src_ip, src_port, payload)) = udp_socket_state.recv() {
                let src_addr = format!("{}:{}", src_ip, src_port);
                Ok((payload, src_addr))
            } else {
                // 暂时返回空数据表示没有收到，或者你可以选择阻塞/报错
                // 为了简单起见，这里模拟非阻塞模式，返回 WouldBlock 错误
                // 但 anyhow 不太好表达 WouldBlock，我们先返回一个特定的错误信息
                anyhow::bail!("No data available");
            }
        } else {
            anyhow::bail!("Socket state not found");
        }
    }
}

fn parse_addr(addr: &str) -> anyhow::Result<(Ipv4Addr, u16)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid address format, expected IP:PORT");
    }

    let ip = parts[0]
        .parse::<Ipv4Addr>()
        .map_err(|_| anyhow::anyhow!("Invalid IP address"))?;
    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| anyhow::anyhow!("Invalid port number"))?;

    Ok((ip, port))
}
