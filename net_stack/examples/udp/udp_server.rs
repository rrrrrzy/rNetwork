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

use std::{thread, time::Duration};

use anyhow::Result;
use clap::Parser;
use net_stack::{
    cli::Args,
    config,
    stack,
    transport::udp::UdpSocket,
};

fn main() -> Result<()> {
    let args = Args::parse();
    let stack_config = config::load_config(&args)?;
    let stack = stack::initialize(&args.iface, stack_config)?;

    // 启动网络栈主循环线程
    let stack_clone = stack.clone();
    thread::spawn(move || {
        if let Err(e) = net_stack::event_loop::run(stack_clone) {
            eprintln!("Event loop error: {:?}", e);
        }
    });

    // 绑定 UDP 端口 8080
    let socket = UdpSocket::bind(stack.clone(), "0.0.0.0:8080")?;
    println!("UDP Server listening on 0.0.0.0:8080");

    loop {
        // 非阻塞接收
        match socket.recv_from() {
            Ok((data, src_addr)) => {
                let msg = String::from_utf8_lossy(&data);
                println!("Received from {}: {}", src_addr, msg);

                // 回显 (Echo)
                let reply = format!("Echo: {}", msg);
                socket.send_to(reply.as_bytes(), &src_addr)?;
            }
            Err(_) => {
                // 没有数据，稍微休眠避免 CPU 空转
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
