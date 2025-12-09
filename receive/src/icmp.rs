use core::hash;
use std::process;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::checksum::simple_checksum;
use crate::config::ICMP_PAYLOAD_SIZE;
use crate::error::ICMPError;

/// 全局 ICMP 序列号计数器(线程安全)
static ICMP_SEQ_COUNTER: AtomicU16 = AtomicU16::new(1);

/// 标准的ICMP协议首部，长度固定为8字节
struct ICMPHeader {
    type_: u8,
    code: u8,
    checksum: u16,
    id: u16,
    seq: u16,
}

impl ICMPHeader {
    fn new() -> Self {
        Self {
            type_: 0x8,                                            // Echo Request
            code: 0x0,                                             // 必须为0
            checksum: 0x0,                                         // 先置为0, 后续重新计算
            id: process::id() as u16, // 进程PID，用于在多进程中区分进程，会被截断为16位
            seq: ICMP_SEQ_COUNTER.fetch_add(1, Ordering::Relaxed), // 随程序自增的序号
        }
    }

    fn set_checksum(&mut self, checksum: u16) {
        self.checksum = checksum;
    }

    fn to_bytes(&self) -> [u8; 8] {
        let [c0, c1] = self.checksum.to_be_bytes();
        let [i0, i1] = self.id.to_be_bytes();
        let [s0, s1] = self.seq.to_be_bytes();
        [self.type_, self.code, c0, c1, i0, i1, s0, s1]
    }

    fn validate(&self) -> Result<(), ICMPError> {
        match self.type_ {
            0x00 | 0x08 => {}
            _ => return Err(ICMPError::ICMP_HEADER_NOT_VALID),
        }
        match self.code {
            0x00 => Ok(()),
            _ => Err(ICMPError::ICMP_HEADER_NOT_VALID),
        }
    }
}

impl TryFrom<Vec<u8>> for ICMPHeader {
    type Error = ICMPError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            // 首部的长度不正确，不为8字节
            Err(ICMPError::ICMP_HEADER_LENGTH_NOT_MATCH)
        } else {
            // 尝试解析为首部
            let [type_, code, c0, c1, i0, i1, s0, s1]: [u8; 8] = value
                .try_into()
                .map_err(|_| ICMPError::ICMP_HEADER_LENGTH_NOT_MATCH)?;

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

struct ICMPPayload {
    time: u32,
    payload: Vec<u8>,
}

impl ICMPPayload {
    /// 负载中需要实现时间戳（标准ping程序的做法），单位毫秒
    fn new() -> Self {
        // payload 部分的 size
        let size = ICMP_PAYLOAD_SIZE - 4;

        let timenow = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u32;

        let mut payload = vec![0u8; size];
        // 循环填充模式数据
        for i in 0..size {
            payload[i] = (i % 256) as u8;
        }

        ICMPPayload {
            time: timenow,
            payload,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + self.payload.len());
        bytes.copy_from_slice(&self.time.to_be_bytes());
        bytes.copy_from_slice(&self.payload);
        bytes
    }

    fn validate(&self) -> Result<(), ICMPError> {
        if self.time
            > SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u32
        {
            Err(ICMPError::ICMP_TIME_NOT_VALID)
        } else {
            Ok(())
        }
    }
}

impl TryFrom<Vec<u8>> for ICMPPayload {
    type Error = ICMPError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            // 首部的长度不正确，小于4字节，不包含我们需要的时间戳
            Err(ICMPError::ICMP_PAYLOAD_LENGTH_NOT_MATCH)
        } else {
            // 这里不检查字段值是否正确，只是尝试解析，字段检查交给另外的函数
            let time = u32::from_be_bytes(
                value[0..4]
                    .try_into()
                    .map_err(|_| ICMPError::ICMP_PAYLOAD_LENGTH_NOT_MATCH)?,
            );
            Ok(Self {
                time,
                payload: value[4..].to_vec(),
            })
        }
    }
}

pub struct ICMP {
    header: ICMPHeader,
    data: ICMPPayload,
}

impl ICMP {
    /// not recommend using this, 只是为了和上面的类保持一致
    pub fn new() -> Self {
        let mut header = ICMPHeader::new();
        let payload = ICMPPayload::new();

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

    /// recommend using
    pub fn send() -> Vec<u8> {
        ICMP::new().to_bytes()
    }

    /// 为 try_from 封装一层的函数
    pub fn recv(value: Vec<u8>) -> Result<Self, ICMPError> {
        ICMP::try_from(value)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(self.header.to_bytes().len() + self.data.to_bytes().len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.data.to_bytes());
        bytes
    }

    pub fn validate(&self) -> Result<(), ICMPError> {
        self.header.validate()?;
        self.data.validate()?;
        Ok(())
    }
}

impl TryFrom<Vec<u8>> for ICMP {
    type Error = ICMPError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < 8 + 4 {
            Err(ICMPError::ICMP_LENGTH_ERR)
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
}
