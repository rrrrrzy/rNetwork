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

use crate::cli::Args;
use crate::stack::StackConfig;
use anyhow::{Context, Result};
use protocol::{ipv4::Ipv4Addr, mac::MacAddr};
use std::fs;
use std::str::FromStr;

pub fn load_config(args: &Args) -> Result<StackConfig> {
    let (ip_str, mac_str) = if let Some(config_path) = &args.config {
        load_from_file(config_path)?
    } else {
        let ip = args
            .ip
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--ip required"))?;
        let mac = args
            .mac
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--mac required"))?;
        (ip, mac)
    };

    let ip = Ipv4Addr::from_str(&ip_str)?;
    let mac = MacAddr::from_str(&mac_str)?;

    Ok(StackConfig { mac, ip })
}

fn load_from_file(path: &std::path::Path) -> Result<(String, String)> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut ip = None;
    let mut mac = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ip" => ip = Some(value.to_string()),
                "mac" => mac = Some(value.to_string()),
                _ => eprintln!("Warning: Unknown config key: {}", key),
            }
        }
    }

    let ip = ip.ok_or_else(|| anyhow::anyhow!("Missing 'ip' in config file"))?;
    let mac = mac.ok_or_else(|| anyhow::anyhow!("Missing 'mac' in config file"))?;

    Ok((ip, mac))
}
