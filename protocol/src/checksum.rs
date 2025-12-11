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

pub struct Crc32 {
    table: [u32; 256],
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc32 {
    pub fn new() -> Self {
        let mut table = [0u32; 256];
        for (i, item) in table.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 == 1 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
            *item = crc;
        }
        Self { table }
    }

    pub fn checksum(&self, buffer: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in buffer {
            let idx = ((crc & 0xFF) ^ byte as u32) as usize;
            crc = (crc >> 8) ^ self.table[idx];
        }
        crc ^ 0xFFFF_FFFF
    }
}

pub fn simple_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = header.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&byte) = chunks.remainder().first() {
        sum += (byte as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}
