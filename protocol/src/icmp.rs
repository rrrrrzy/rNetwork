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

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::checksum::simple_checksum;
use crate::error::IcmpParseError;

#[derive(Clone, Copy, Debug, Hash)]
/// 标准的ICMP协议首部，长度固定为8字节
pub struct ICMPHeader {
    pub type_: u8,
    pub code: u8,
    pub checksum: u16,
    pub id: u16,
    pub seq: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpType {
    Request,
    Reply,
    Unknown,
}

impl IcmpType {
    pub fn type_code(self) -> u8 {
        match self {
            IcmpType::Request => 0x8,
            IcmpType::Reply => 0x0,
            IcmpType::Unknown => 0x1,
        }
    }

    pub fn parse(type_: u8) -> IcmpType {
        match type_ {
            0x8 => IcmpType::Request,
            0x0 => IcmpType::Reply,
            _ => IcmpType::Unknown,
        }
    }
}

impl ICMPHeader {
    pub fn new(type_: IcmpType, code: u8, id: u16, seq: u16) -> Self {
        Self {
            type_: type_.type_code(),
            code,
            checksum: 0x0,
            id,
            seq,
        }
    }

    pub fn set_checksum(&mut self, checksum: u16) {
        self.checksum = checksum;
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let [c0, c1] = self.checksum.to_be_bytes();
        let [i0, i1] = self.id.to_be_bytes();
        let [s0, s1] = self.seq.to_be_bytes();
        [self.type_, self.code, c0, c1, i0, i1, s0, s1]
    }

    pub fn validate(&self) -> Result<(), IcmpParseError> {
        match self.type_ {
            0x00 | 0x08 => {}
            _ => return Err(IcmpParseError::InvalidIcmpHeader),
        }
        match self.code {
            0x00 => Ok(()),
            _ => Err(IcmpParseError::InvalidIcmpHeader),
        }
    }
}

impl TryFrom<Vec<u8>> for ICMPHeader {
    type Error = IcmpParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            // 首部的长度不正确，不为8字节
            Err(IcmpParseError::IcmpHeaderLengthNotMatch)
        } else {
            // 尝试解析为首部
            let [type_, code, c0, c1, i0, i1, s0, s1]: [u8; 8] = value
                .try_into()
                .map_err(|_| IcmpParseError::IcmpHeaderLengthNotMatch)?;

            // 这里不检查字段值是否正确，只是尝试解析，字段检查交给另外的函数

            Ok(Self {
                type_,
                code,
                checksum: u16::from_be_bytes([c0, c1]),
                id: u16::from_be_bytes([i0, i1]),
                seq: u16::from_be_bytes([s0, s1]),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct ICMPPayload {
    pub time: u32,
    pub payload: Vec<u8>,
}

impl ICMPPayload {
    /// 纯构造函数：将传入的时间戳和负载数据封装为 ICMPPayload
    /// payload 参数使用 &[u8]，内部会进行拷贝以获得所有权
    pub fn new(time: u32, payload: &[u8]) -> Self {
        Self {
            time,
            payload: payload.to_vec(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + self.payload.len());
        bytes.extend_from_slice(&self.time.to_be_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn validate(&self) -> Result<(), IcmpParseError> {
        // if self.time
        //     > SystemTime::now()
        //         .duration_since(UNIX_EPOCH)
        //         .expect("Time went backwards")
        //         .as_millis() as u32
        // {
        //     Err(IcmpParseError::InvalidIcmpTime)
        // } else {
        //     Ok(())
        // }
        // Linux中，内核一般不会检查payload的时间是否合理，只有在上层应用中才会检查
        Ok(())
    }
}

impl TryFrom<Vec<u8>> for ICMPPayload {
    type Error = IcmpParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            // 载荷的长度不正确，小于4字节，不包含我们需要的时间戳
            Err(IcmpParseError::IcmpPayloadLengthNotMatch)
        } else {
            // 这里不检查字段值是否正确，只是尝试解析，字段检查交给另外的函数
            let time = u32::from_be_bytes(
                value[0..4]
                    .try_into()
                    .map_err(|_| IcmpParseError::IcmpPayloadLengthNotMatch)?,
            );
            Ok(Self {
                time,
                payload: value[4..].to_vec(),
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct ICMP {
    pub header: ICMPHeader,
    pub data: ICMPPayload,
}

impl ICMP {
    /// 构造一个ICMP报文时，必须传入构造的首部参数和载荷内容
    pub fn new(type_: IcmpType, code: u8, id: u16, seq: u16, time: u32, payload: &[u8]) -> Self {
        let mut header = ICMPHeader::new(type_, code, id, seq);
        let payload = ICMPPayload::new(time, payload);

        // get raw datas
        let header_raw = &header.to_bytes();
        let payload_raw = &payload.to_bytes();

        // get total raw content
        let mut raw_cont = Vec::with_capacity(header_raw.len() + payload_raw.len());
        raw_cont.extend_from_slice(header_raw);
        raw_cont.extend_from_slice(payload_raw);

        // calculate simple sum and set to header
        header.set_checksum(simple_checksum(&raw_cont));

        Self {
            header,
            data: payload,
        }
    }

    pub fn parse(value: &[u8]) -> Result<Self, IcmpParseError> {
        if value.len() < 8 + 4 {
            Err(IcmpParseError::IcmpLengthErr)
        } else {
            let head = ICMPHeader::try_from(value[0..8].to_vec())?;
            let payload = ICMPPayload::try_from(value[8..].to_vec())?;
            let tmp = Self {
                header: head,
                data: payload,
            };
            tmp.validate()?;
            Ok(tmp)
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(self.header.to_bytes().len() + self.data.to_bytes().len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.data.to_bytes());
        bytes
    }

    pub fn validate(&self) -> Result<(), IcmpParseError> {
        self.header.validate()?;
        self.data.validate()?;
        Ok(())
    }
}

impl TryFrom<Vec<u8>> for ICMP {
    type Error = IcmpParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::parse(value.as_slice())
    }
}

impl fmt::Display for ICMP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icmp_type = IcmpType::parse(self.header.type_);
        match icmp_type {
            IcmpType::Request => {
                write!(
                    f,
                    "ICMP Echo Request: id={}, seq={}, payload_len={}",
                    self.header.id,
                    self.header.seq,
                    self.data.payload.len()
                )
            }
            IcmpType::Reply => {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as u32;
                let rtt = current_time.saturating_sub(self.data.time);
                write!(
                    f,
                    "ICMP Echo Reply: id={}, seq={}, rtt={}ms, payload_len={}",
                    self.header.id,
                    self.header.seq,
                    rtt,
                    self.data.payload.len()
                )
            }
            IcmpType::Unknown => {
                write!(
                    f,
                    "ICMP Unknown Type: type={}, code={}, id={}, seq={}",
                    self.header.type_, self.header.code, self.header.id, self.header.seq
                )
            }
        }
    }
}
