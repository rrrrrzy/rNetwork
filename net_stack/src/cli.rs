// Copyright (C) 2025 rrrrrzy
// SPDX-License-Identifier: GPL-3.0-or-later
//
// --------------------------------------------------
// 致敬所有在深夜调试代码的灵魂。
// 即便 Bug 如山,我亦往矣。
// --------------------------------------------------
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Network interface to use
    #[arg(short, long)]
    pub iface: String,

    /// IP address of this stack (optional if using config file)
    #[arg(long)]
    pub ip: Option<String>,

    /// MAC address of this stack (optional if using config file)
    #[arg(long)]
    pub mac: Option<String>,

    /// Configuration file path (format: ip=x.x.x.x\nmac=xx:xx:xx:xx:xx:xx)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Ping target IP address
    #[arg(long)]
    pub ping: Option<String>,

    /// Target MAC address for ping (temporary until ARP is fully implemented)
    #[arg(long)]
    pub target_mac: Option<String>,
}
