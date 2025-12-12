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

mod cli;
mod config;
mod event_loop;
mod handlers;
mod stack;
mod transport;

use anyhow::Result;
use clap::Parser;
use cli::Args;

fn main() -> Result<()> {
    let args = Args::parse();

    // 从配置文件或命令行参数获取 IP 和 MAC
    let stack_config = config::load_config(&args)?;

    let stack = stack::initialize(&args.iface, stack_config)?;

    if let Some(target_ip_str) = args.ping {
        event_loop::ping(&target_ip_str, &stack)?;
    }

    event_loop::run(stack)?;

    Ok(())
}
