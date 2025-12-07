use std::{net::Ipv4Addr, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::mac::MacAddr;

const DEFAULT_ETHERTYPE: u16 = 0x0080;
const DEFAULT_DEST_MAC_STR: &str = "44:87:fc:d6:bd:8c";
const DEFAULT_SRC_MAC_STR: &str = "44:87:fc:d6:bf:91";
const DEFAULT_TARGET_MAC_STR: &str = "00:00:00:00:00:00";

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ArpMode {
    Request,
    Reply,
}

#[derive(Parser)]
#[command(
    name = "ethernet_frame_send",
    version,
    about = "使用 libpcap 构造并发送原始以太网帧"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// 列出 libpcap 能打开的所有接口
    List,
    /// 从文件读取载荷并持续发送自定义以太网帧
    Send(SendArgs),
}

#[derive(Args, Clone)]
pub struct SendArgs {
    /// `list` 子命令输出的接口名称（如 en0）
    #[arg(short, long)]
    pub interface: String,
    /// 作为帧载荷的数据文件路径
    #[arg(short, long, default_value = "data.txt")]
    pub data: PathBuf,
    /// 目的 MAC 地址
    #[arg(long, default_value = DEFAULT_DEST_MAC_STR)]
    pub dest_mac: MacAddr,
    /// 源 MAC 地址
    #[arg(long, default_value = DEFAULT_SRC_MAC_STR)]
    pub src_mac: MacAddr,
    /// EtherType 字段（十六进制）
    #[arg(long, default_value_t = DEFAULT_ETHERTYPE)]
    pub ethertype: u16,
    /// 小于 46 字节时自动补零
    #[arg(long)]
    pub pad: bool,
    /// 将帧 payload 构造成 ARP 报文
    #[arg(long, value_enum, conflicts_with = "ipv4")]
    pub arp_mode: Option<ArpMode>,
    /// 是否将数据封装为 IPv4 报文
    #[arg(long)]
    pub ipv4: bool,
    /// IPv4 源地址（仅在 --ipv4 启用时生效）
    #[arg(long, default_value = "10.13.80.43")]
    pub src_ip: Ipv4Addr,
    /// IPv4 目的地址（仅在 --ipv4 启用时生效）
    #[arg(long, default_value = "255.255.255.255")]
    pub dst_ip: Ipv4Addr,
    /// ARP 目的 IP 地址（仅在 --arp-mode 启用时生效）
    #[arg(long, default_value = "10.0.0.1", requires = "arp_mode")]
    pub arp_target_ip: Ipv4Addr,
    /// ARP 目的 MAC 地址（仅在 --arp-mode 启用时生效）
    #[arg(long, default_value = DEFAULT_TARGET_MAC_STR, requires = "arp_mode")]
    pub arp_target_mac: MacAddr,
    /// IPv4 TTL 字段
    #[arg(long, default_value_t = 64)]
    pub ttl: u8,
    /// IPv4 服务类型 (TOS)
    #[arg(long, default_value_t = 0xFE)]
    pub tos: u8,
    /// IPv4 上层协议号
    #[arg(long, default_value_t = 6)]
    pub protocol: u8,
    /// IPv4 载荷分片大小（字节）
    #[arg(long, default_value_t = 1400)]
    pub fragment_size: usize,
    /// IPv4 报文标识符
    #[arg(long, default_value_t = 0)]
    pub ip_id: u16,
    /// 设置 DF 标志，禁止进一步分片
    #[arg(long)]
    pub dont_fragment: bool,
    /// 要发送的帧数（默认无限循环）
    #[arg(long)]
    pub count: Option<u64>,
    /// 帧之间的等待时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    pub interval_ms: u64,
    /// 传递给 libpcap 的读取超时时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    pub timeout_ms: i32,
}
