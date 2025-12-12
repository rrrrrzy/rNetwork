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

use std::borrow::Cow;
use std::{error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpParseError {
    IcmpLengthErr,
    IcmpHeaderLengthNotMatch,
    IcmpPayloadLengthNotMatch,
    InvalidIcmpHeader,
    InvalidIcmpTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv4ParseError {
    InvalidFormat,
    InvalidOctet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv4HeaderParseError {
    InvalidVersion,
    InvalidHeaderLength,
    InvalidFlags,
    InvalidOffset, // do not support currently
    InvalidTimeToLive,
    InvalidChecksum,
}

impl fmt::Display for Ipv4HeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChecksum => write!(f, "IPv4 header checksum validation failed"),
            Self::InvalidVersion => write!(f, "IPv4 version must be 4"),
            Self::InvalidHeaderLength => write!(f, "IPv4 header length is invalid"),
            Self::InvalidTimeToLive => write!(f, "IPv4 TTL cannot be 0"),
            Self::InvalidFlags => write!(f, "IPv4 flags field is invalid"),
            Self::InvalidOffset => write!(f, "IPv4 fragment offset is invalid"),
        }
    }
}

impl error::Error for Ipv4HeaderParseError {}

#[derive(Debug, Clone)]
pub struct MacParseError(pub Cow<'static, str>);

#[derive(Debug, Clone)]
pub enum EthernetParseError {
    PacketTooShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpParseError {
    InvalidArpFixedLength,
    InvalidArpHardwareLength,
    InvalidArpProtocolLength,
}

impl fmt::Display for ArpParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArpParseError::InvalidArpFixedLength => {
                write!(f, "ARP packet length is too short")
            }
            ArpParseError::InvalidArpHardwareLength => {
                write!(f, "Unsupported ARP hardware address length")
            }
            ArpParseError::InvalidArpProtocolLength => {
                write!(f, "Unsupported ARP protocol address length")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpParseError {
    InvalidUdpLen,
    InvalidChecksum,
}

impl fmt::Display for UdpParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UdpParseError::InvalidUdpLen => write!(f, "UDP packet length is invalid"),
            UdpParseError::InvalidChecksum => write!(f, "UDP checksum validation failed"),
        }
    }
}

impl error::Error for UdpParseError {}
