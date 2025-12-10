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

use static_assertions::const_assert;

/// ICMP的数据部分大小，单位字节，必须 4 字节对齐，最小长度为 4 字节（时间戳）
pub static ICMP_PAYLOAD_SIZE: usize = 1024;
const_assert!(ICMP_PAYLOAD_SIZE % 32 == 0 && ICMP_PAYLOAD_SIZE >= 32);
