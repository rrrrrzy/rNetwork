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

mod arp;
mod cli;
mod config;
mod ethernet;
mod ipv4;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::ethernet::{list_adapters, receive_packets};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => list_adapters(),
        Command::Receive(args) => receive_packets(args),
    }
}
