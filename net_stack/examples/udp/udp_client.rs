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

use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

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

    // 绑定一个随机端口 (或者指定端口)
    let socket = UdpSocket::bind(stack.clone(), "0.0.0.0:12345")?;
    println!("UDP Client bound to 0.0.0.0:12345");

    // 目标地址 (假设 Server 在同一网段的另一台机器，或者本机测试)
    // 注意：如果是本机测试，需要确保 Server 和 Client 绑定不同的端口
    let target = "192.168.31.223:8080"; // 请根据实际情况修改

    println!("Enter message to send to {} (type 'quit' to exit):", target);

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let msg = input.trim();

        if msg == "quit" {
            break;
        }

        // 发送消息
        socket.send_to(msg.as_bytes(), target)?;

        // 尝试接收回复 (带超时重试)
        let mut retries = 0;
        loop {
            match socket.recv_from() {
                Ok((data, src_addr)) => {
                    let reply = String::from_utf8_lossy(&data);
                    println!("Received reply from {}: {}", src_addr, reply);
                    break;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(100));
                    retries += 1;
                    if retries > 10 {
                        println!("No reply received (timeout)");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
