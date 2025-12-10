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

use std::{borrow::Cow, fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    pub const fn from_raw(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub const fn broadcast() -> Self {
        Self([0xFF; 6])
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl FromStr for MacAddr {
    type Err = MacParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split([':', '-']).collect();
        if parts.len() != 6 {
            return Err(MacParseError(Cow::Owned(format!(
                "expected 6 octets, got {}",
                parts.len()
            ))));
        }
        let mut bytes = [0u8; 6];
        for (idx, part) in parts.iter().enumerate() {
            bytes[idx] = u8::from_str_radix(part, 16)
                .map_err(|_| MacParseError(Cow::Owned(format!("invalid octet '{part}'"))))?;
        }
        Ok(Self(bytes))
    }
}

#[derive(Debug, Clone)]
pub struct MacParseError(Cow<'static, str>);

impl fmt::Display for MacParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MacParseError {}
