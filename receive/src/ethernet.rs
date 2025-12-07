use std::fs::File;
use std::io::{BufWriter, Write};

use anyhow::{Context, Result};
use pcap::{Active, Capture, Device, Error as PcapError, Packet};

use crate::arp::ArpProcessor;
use crate::cli::ReceiveArgs;
use crate::checksum::Crc32;
use crate::ipv4::Ipv4Processor;
use crate::mac::MacAddr;

const MIN_PAYLOAD: usize = 46;
const MAX_PAYLOAD: usize = 1500;
const HEADER_LEN: usize = 14;
const CRC_LEN: usize = 4;
const DEFAULT_DEST_MAC: MacAddr = MacAddr::from_raw([0x44, 0x87, 0xFC, 0xD6, 0xBD, 0x8C]);

pub fn list_adapters() -> Result<()> {
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

pub fn receive_packets(args: ReceiveArgs) -> Result<()> {
    let mut capture = open_interface(&args.interface, args.timeout_ms)?;
    let mut writer = BufWriter::new(
        File::create(&args.output)
            .with_context(|| format!("无法创建输出文件 {:?}", args.output))?,
    );
    let mut ipv4 = Ipv4Processor::new(args.accept_ip, &args.ip_output)
        .with_context(|| format!("初始化 IPv4 输出文件 {:?} 失败", args.ip_output))?;
    let mut arp = ArpProcessor::new(ipv4.allowed_slice().to_vec());

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
    println!("目的 IPv4 白名单：{}", ipv4.allowed_destinations());

    let crc = Crc32::new();
    let mut displayed = 0u64;
    loop {
        match capture.next_packet() {
            Ok(packet) => {
                if handle_packet(
                    &packet,
                    displayed + 1,
                    &accepted,
                    &crc,
                    &mut writer,
                    &mut ipv4,
                    &mut arp,
                )? {
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

fn handle_packet(
    packet: &Packet,
    ordinal: u64,
    accepted: &[MacAddr],
    crc: &Crc32,
    writer: &mut BufWriter<File>,
    ipv4: &mut Ipv4Processor,
    arp: &mut ArpProcessor,
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
    print_payload_preview(ethertype, payload);
    let pad_hint = payload.last().copied().unwrap_or(0);
    println!("补零提示字节: {pad_hint}");

    if ethertype == 0x0800 {
        ipv4.process(payload)?;
    } else if ethertype == 0x0806 {
        arp.process(payload)?;
    }

    println!("----------------------");

    writer.write_all(format!("{ordinal}:     ").as_bytes())?;
    writer.write_all(payload)?;
    writer.write_all(b"\n")?;
    writer.flush()?;

    Ok(true)
}

fn print_payload_preview(ethertype: u16, payload: &[u8]) {
    if ethertype == 0x0800 && payload.len() >= 20 {
        let ihl_words = payload[0] & 0x0F;
        let header_len = (ihl_words as usize) * 4;
        if header_len <= payload.len() {
            let data = &payload[header_len..];
            if try_utf8(data) {
                return;
            }
            print_bytes(data);
            return;
        }
    }
    if try_utf8(payload) {
        return;
    }
    print_bytes(payload);
}

fn try_utf8(data: &[u8]) -> bool {
    match std::str::from_utf8(data) {
        Ok(text) => {
            println!("数据内容 (UTF-8): {text}");
            true
        }
        Err(_) => false,
    }
}

fn print_bytes(bytes: &[u8]) {
    print!("数据内容: ");
    for &byte in bytes {
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
    println!("(UTF-8 解析失败，已回退到逐字节转储)");
}
