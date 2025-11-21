use std::{
    borrow::Cow,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use pcap::{Active, Capture, Device, Error as PcapError, Packet};

const MIN_PAYLOAD: usize = 46;
const MAX_PAYLOAD: usize = 1500;
const HEADER_LEN: usize = 14;
const CRC_LEN: usize = 4;
const DEFAULT_ETHERTYPE: u16 = 0x0080;
const DEFAULT_DEST_MAC_STR: &str = "44:87:fc:d6:bd:8c";
const DEFAULT_SRC_MAC_STR: &str = "44:87:fc:d6:bf:91";
const DEFAULT_DEST_MAC: MacAddr = MacAddr::from_raw([0x44, 0x87, 0xFC, 0xD6, 0xBD, 0x8C]);

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => list_adapters(),
        Command::Send(args) => send_packets(args),
        Command::Receive(args) => receive_packets(args),
    }
}

#[derive(Parser)]
#[command(
    name = "ethernet_frame",
    version,
    about = "使用 libpcap 构造并分析原始以太网帧"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 列出 libpcap 能打开的所有接口
    List,
    /// 从文件读取载荷并持续发送自定义以太网帧
    Send(SendArgs),
    /// 抓取帧、按目的 MAC 过滤并保存载荷
    Receive(ReceiveArgs),
}

#[derive(Args)]
struct SendArgs {
    /// `list` 子命令输出的接口名称（如 en0）
    #[arg(short, long)]
    interface: String,
    /// 作为帧载荷的数据文件路径
    #[arg(short, long, default_value = "data.txt")]
    data: PathBuf,
    /// 目的 MAC 地址
    #[arg(long, default_value = DEFAULT_DEST_MAC_STR)]
    dest_mac: MacAddr,
    /// 源 MAC 地址
    #[arg(long, default_value = DEFAULT_SRC_MAC_STR)]
    src_mac: MacAddr,
    /// EtherType 字段（十六进制）
    #[arg(long, default_value_t = DEFAULT_ETHERTYPE)]
    ethertype: u16,
    /// 小于 46 字节时自动补零
    #[arg(long)]
    pad: bool,
    /// 要发送的帧数（默认无限循环）
    #[arg(long)]
    count: Option<u64>,
    /// 帧之间的等待时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    interval_ms: u64,
    /// 传递给 libpcap 的读取超时时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    timeout_ms: i32,
}

#[derive(Args, Clone)]
struct ReceiveArgs {
    /// `list` 子命令输出的接口名称（如 en0）
    #[arg(short, long)]
    interface: String,
    /// 用于保存载荷的输出文件
    #[arg(long, default_value = "recv.txt")]
    output: PathBuf,
    /// 额外允许的目的 MAC（可用逗号分隔多个）
    #[arg(long, value_name = "MAC", value_delimiter = ',')]
    accept: Vec<MacAddr>,
    /// 接收指定数量的匹配帧后停止（默认持续抓取）
    #[arg(long)]
    limit: Option<u64>,
    /// 传递给 libpcap 的读取超时时间（毫秒）
    #[arg(long, default_value_t = 1000)]
    timeout_ms: i32,
}

fn list_adapters() -> Result<()> {
    let devices = Device::list().context("枚举网卡接口失败")?;
    if devices.is_empty() {
        println!("libpcap 未报告任何可用接口。");
        return Ok(());
    }
    for (idx, dev) in devices.iter().enumerate() {
        match dev.desc.as_deref() {
            Some(desc) => println!("{}. {} ({})", idx + 1, dev.name, desc),
            None => println!("{}. {} (无描述)", idx + 1, dev.name),
        }
    }
    Ok(())
}

fn send_packets(args: SendArgs) -> Result<()> {
    let mut payload =
        fs::read(&args.data).with_context(|| format!("无法从 {:?} 读取载荷", args.data))?;
    enforce_payload_rules(&mut payload, args.pad)?;

    let crc = Crc32::new().checksum(&payload);
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len() + CRC_LEN);
    frame.extend_from_slice(args.dest_mac.as_bytes());
    frame.extend_from_slice(args.src_mac.as_bytes());
    frame.extend_from_slice(&args.ethertype.to_be_bytes());
    frame.extend_from_slice(&payload);
    frame.extend_from_slice(&crc.to_le_bytes());

    let mut capture = open_interface(&args.interface, args.timeout_ms)?;

    let mut sent = 0u64;
    loop {
        capture
            .sendpacket(frame.as_slice())
            .with_context(|| "pcap 无法发送构造的帧")?;
        sent += 1;

        if args.count.is_some_and(|max| sent >= max) {
            break;
        }

        if args.interval_ms > 0 {
            thread::sleep(Duration::from_millis(args.interval_ms));
        }
    }

    println!(
        "已发送 {sent} 个帧，总长度 {} 字节（载荷 {} + 帧头 {} + CRC {}）。",
        frame.len(),
        payload.len(),
        HEADER_LEN,
        CRC_LEN
    );
    Ok(())
}

fn receive_packets(args: ReceiveArgs) -> Result<()> {
    let mut capture = open_interface(&args.interface, args.timeout_ms)?;
    let mut writer = BufWriter::new(
        File::create(&args.output)
            .with_context(|| format!("无法创建输出文件 {:?}", args.output))?,
    );

    let mut accepted = if args.accept.is_empty() {
        vec![MacAddr::broadcast(), DEFAULT_DEST_MAC]
    } else {
        args.accept.clone()
    };
    accepted.sort();
    accepted.dedup();

    println!(
        "目的 MAC 白名单：{}",
        accepted
            .iter()
            .map(|mac| mac.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let crc = Crc32::new();
    let mut displayed = 0u64;
    loop {
        match capture.next_packet() {
            Ok(packet) => {
                if handle_packet(&packet, displayed + 1, &accepted, &crc, &mut writer)? {
                    displayed += 1;
                    if let Some(limit) = args.limit.filter(|&limit| displayed >= limit) {
                        println!("已达到 {limit} 个匹配帧的抓包上限。");
                        break;
                    }
                }
            }
            Err(PcapError::TimeoutExpired) => continue,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

fn open_interface(name: &str, timeout_ms: i32) -> Result<Capture<Active>> {
    Capture::from_device(name)
        .with_context(|| format!("找不到接口 {name}"))?
        .promisc(true)
        .snaplen(65535)
        .timeout(timeout_ms)
        .open()
        .with_context(|| format!("打开接口 {name} 失败"))
}

fn enforce_payload_rules(payload: &mut Vec<u8>, pad: bool) -> Result<()> {
    if payload.len() > MAX_PAYLOAD {
        bail!(
            "载荷大小 {} 超出以太网最大 {} 字节限制",
            payload.len(),
            MAX_PAYLOAD
        );
    }

    if payload.len() < MIN_PAYLOAD {
        if pad {
            payload.resize(MIN_PAYLOAD, 0);
        } else {
            bail!(
                "载荷大小 {} 小于以太网最小 {} 字节（使用 --pad 自动补零）",
                payload.len(),
                MIN_PAYLOAD
            );
        }
    }
    Ok(())
}

fn handle_packet(
    packet: &Packet,
    ordinal: u64,
    accepted: &[MacAddr],
    crc: &Crc32,
    writer: &mut BufWriter<File>,
) -> Result<bool> {
    if packet.data.len() < HEADER_LEN + CRC_LEN {
        println!("检测到长度仅 {} 字节的畸形帧，已忽略。", packet.data.len());
        return Ok(false);
    }

    let dest = MacAddr::from_slice(&packet.data[0..6]);
    if !accepted.is_empty() && !accepted.contains(&dest) {
        println!("目的地址 {dest} 不在白名单，已丢弃。");
        return Ok(false);
    }

    let src = MacAddr::from_slice(&packet.data[6..12]);
    let ethertype = u16::from_be_bytes([packet.data[12], packet.data[13]]);
    let crc_range_start = packet.data.len() - CRC_LEN;
    let payload = &packet.data[HEADER_LEN..crc_range_start];
    let frame_crc = u32::from_le_bytes(packet.data[crc_range_start..].try_into().unwrap());

    if payload.len() < MIN_PAYLOAD || payload.len() > MAX_PAYLOAD {
        println!("数据长度 {} 不满足以太网要求，已跳过该帧。", payload.len());
        return Ok(false);
    }

    let computed_crc = crc.checksum(payload);
    let crc_ok = computed_crc == frame_crc;

    println!("----------------------------");
    println!("捕获第 {ordinal} 个数据帧");
    println!("捕获时间戳: {}", packet.header.ts.tv_sec);
    println!("帧总长度: {}", packet.header.len);
    println!("以太网头长度: {}", HEADER_LEN);
    println!("数据区长度: {}", payload.len());
    println!("-----Ethernet protocol-------");
    println!("EtherType: {ethertype:#06x}");
    println!("源 MAC: {src}");
    println!("目的 MAC: {dest}");
    println!(
        "CRC32（载荷）: 0x{frame_crc:08x} ({})",
        if crc_ok { "匹配" } else { "不一致" }
    );
    print!("数据内容: ");
    for &byte in payload {
        if byte.is_ascii_graphic() || byte == b' ' {
            print!("{}", byte as char);
        } else if byte == b'\n' {
            print!("\\n");
        } else if byte == b'\r' {
            print!("\\r");
        } else {
            print!("\\x{:02X}", byte);
        }
    }
    println!();
    let pad_hint = payload.last().copied().unwrap_or(0);
    println!("补零提示字节: {pad_hint}");
    println!("----------------------");

    writer.write_all(format!("{ordinal}:     ").as_bytes())?;
    writer.write_all(payload)?;
    writer.write_all(b"\n")?;
    writer.flush()?;

    Ok(true)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MacAddr([u8; 6]);

impl MacAddr {
    const fn from_raw(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    fn broadcast() -> Self {
        Self([0xFF; 6])
    }

    fn from_slice(slice: &[u8]) -> Self {
        let mut bytes = [0u8; 6];
        bytes.copy_from_slice(slice);
        Self(bytes)
    }
}

impl std::fmt::Display for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl std::str::FromStr for MacAddr {
    type Err = MacParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split([':', '-']).collect();
        if parts.len() != 6 {
            return Err(MacParseError(Cow::Owned(format!(
                "expected 6 octets, got {}",
                parts.len()
            ))));
        }
        let mut bytes = [0u8; 6];
        for (idx, part) in parts.iter().enumerate() {
            bytes[idx] = u8::from_str_radix(part, 16)
                .map_err(|_| MacParseError(Cow::Owned(format!("invalid octet '{part}'"))))?;
        }
        Ok(Self(bytes))
    }
}

#[derive(Debug, Clone)]
struct MacParseError(Cow<'static, str>);

impl std::fmt::Display for MacParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MacParseError {}

struct Crc32 {
    table: [u32; 256],
}

impl Crc32 {
    fn new() -> Self {
        let mut table = [0u32; 256];
        for (i, item) in table.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 == 1 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
            *item = crc;
        }
        Self { table }
    }

    fn checksum(&self, buffer: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in buffer {
            let idx = ((crc & 0xFF) ^ byte as u32) as usize;
            crc = (crc >> 8) ^ self.table[idx];
        }
        crc ^ 0xFFFF_FFFF
    }
}
