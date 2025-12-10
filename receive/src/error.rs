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

pub enum ICMPError {
    ICMP_LENGTH_ERR,
    ICMP_HEADER_LENGTH_NOT_MATCH,
    ICMP_PAYLOAD_LENGTH_NOT_MATCH,
    ICMP_HEADER_NOT_VALID,
    ICMP_TIME_NOT_VALID,
}
