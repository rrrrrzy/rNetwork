use std::{net::Ipv4Addr, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use crate::mac::MacAddr;

#[derive(Parser)]
#[command(
    name = "ethernet_frame_receive",
    version,
    about = "使用 libpcap 抓取原始以太网帧"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// 列出 libpcap 能打开的所有接口
    List,
    /// 抓取帧、按目的 MAC 过滤并保存载荷
    Receive(ReceiveArgs),
}

#[derive(Args, Clone)]
pub struct ReceiveArgs {
    /// `list` 子命令输出的接口名称（如 en0）
    #[arg(short, long)]
    pub interface: String,
    /// 用于保存以太网载荷日志的输出文件
    #[arg(long, default_value = "recv.txt")]
    pub output: PathBuf,
    /// IPv4 数据重组后写入的文件
    #[arg(long, default_value = "ip_data.bin")]
    pub ip_output: PathBuf,
    /// 额外允许的目的 MAC（可用逗号分隔多个）
    #[arg(long, value_name = "MAC", value_delimiter = ',')]
    pub accept: Vec<MacAddr>,
    /// 允许的目的 IPv4 地址（可用逗号分隔多个）
    #[arg(long, value_name = "IPv4", value_delimiter = ',')]
    pub accept_ip: Vec<Ipv4Addr>,
    /// 接收指定数量的匹配帧后停止（默认持续抓取）
    #[arg(long)]
    pub limit: Option<u64>,
    /// 传递给 libpcap 的读取超时时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    pub timeout_ms: i32,
}
