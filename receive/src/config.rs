use static_assertions::const_assert;

/// ICMP的数据部分大小，单位字节，必须 4 字节对齐，最小长度为 4 字节（时间戳）
pub static ICMP_PAYLOAD_SIZE: usize = 1024;
const_assert!(ICMP_PAYLOAD_SIZE % 32 == 0 && ICMP_PAYLOAD_SIZE >= 32);
