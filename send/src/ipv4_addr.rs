use std::fmt;
use std::str::FromStr;

/// 自定义 IPv4 地址类型，减少对标准库的依赖
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ipv4Addr([u8; 4]);

impl Ipv4Addr {
    /// 从四个字节构造 IPv4 地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// 从字节数组构造
    pub const fn from_octets(octets: [u8; 4]) -> Self {
        Self(octets)
    }

    /// 获取字节数组
    pub const fn octets(&self) -> [u8; 4] {
        self.0
    }

    /// 广播地址 255.255.255.255
    pub const fn broadcast() -> Self {
        Self([255, 255, 255, 255])
    }

    /// 未指定地址 0.0.0.0
    pub const fn unspecified() -> Self {
        Self([0, 0, 0, 0])
    }

    /// 本地回环地址 127.0.0.1
    pub const fn localhost() -> Self {
        Self([127, 0, 0, 1])
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl FromStr for Ipv4Addr {
    type Err = Ipv4ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(Ipv4ParseError::InvalidFormat);
        }

        let mut octets = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            octets[i] = part.parse().map_err(|_| Ipv4ParseError::InvalidOctet)?;
        }

        Ok(Self(octets))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv4ParseError {
    InvalidFormat,
    InvalidOctet,
}

impl fmt::Display for Ipv4ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "IPv4 地址格式错误，应为 a.b.c.d"),
            Self::InvalidOctet => write!(f, "IPv4 地址字节必须在 0-255 之间"),
        }
    }
}

impl std::error::Error for Ipv4ParseError {}
