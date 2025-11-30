use std::net::Ipv4Addr;

use anyhow::{Result, bail, ensure};

const IPV4_HEADER_LEN: usize = 60;
const IPV4_IHL_WORDS: u8 = (IPV4_HEADER_LEN / 4) as u8;
const MAX_ETHERNET_PAYLOAD: usize = 1500;
const MAX_FRAGMENT_SIZE: usize = MAX_ETHERNET_PAYLOAD - IPV4_HEADER_LEN;
const MAX_DATAGRAM_DATA: usize = u16::MAX as usize - IPV4_HEADER_LEN;

#[derive(Clone, Debug)]
pub struct Ipv4Config {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub ttl: u8,
    pub tos: u8,
    pub protocol: u8,
    pub fragment_size: usize,
    pub identification: u16,
    pub dont_fragment: bool,
}

pub fn build_ipv4_payloads(data: &[u8], cfg: &Ipv4Config) -> Result<Vec<Vec<u8>>> {
    if data.is_empty() {
        bail!("输入文件为空，无法封装 IPv4 报文。");
    }

    ensure!(cfg.fragment_size > 0, "IPv4 分片大小必须大于 0。");
    ensure!(
        cfg.fragment_size <= MAX_FRAGMENT_SIZE,
        "IPv4 分片大小 {0} 超出以太网最大载荷 {MAX_FRAGMENT_SIZE}。",
        cfg.fragment_size
    );
    ensure!(
        cfg.fragment_size % 8 == 0,
        "IPv4 分片大小必须是 8 的倍数，以满足分片偏移要求。"
    );
    ensure!(
        data.len() <= MAX_DATAGRAM_DATA,
        "当前实现仅支持单个 IPv4 报文，最大净载荷 {MAX_DATAGRAM_DATA} 字节。"
    );

    let mut payloads = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let chunk_len = remaining.min(cfg.fragment_size);
        let more = offset + chunk_len < data.len();
        let fragment = &data[offset..offset + chunk_len];
        payloads.push(assemble_fragment(fragment, more, offset, cfg));
        offset += chunk_len;
    }

    Ok(payloads)
}

fn assemble_fragment(fragment: &[u8], more: bool, offset: usize, cfg: &Ipv4Config) -> Vec<u8> {
    let mut packet = vec![0u8; IPV4_HEADER_LEN];
    packet.extend_from_slice(fragment);

    packet[0] = (4 << 4) | IPV4_IHL_WORDS;
    packet[1] = cfg.tos;
    let total_length = (IPV4_HEADER_LEN + fragment.len()) as u16;
    packet[2..4].copy_from_slice(&total_length.to_be_bytes());
    packet[4..6].copy_from_slice(&cfg.identification.to_be_bytes());

    let mut flags_offset = ((offset / 8) as u16) & 0x1FFF;
    if more {
        flags_offset |= 0x2000;
    }
    if cfg.dont_fragment {
        flags_offset |= 0x4000;
    }
    packet[6..8].copy_from_slice(&flags_offset.to_be_bytes());

    packet[8] = cfg.ttl;
    packet[9] = cfg.protocol;
    packet[10..12].fill(0);
    packet[12..16].copy_from_slice(&cfg.src_ip.octets());
    packet[16..20].copy_from_slice(&cfg.dst_ip.octets());

    let checksum = ipv4_checksum(&packet[..IPV4_HEADER_LEN]);
    packet[10..12].copy_from_slice(&checksum.to_be_bytes());

    packet
}

fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = header.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&byte) = chunks.remainder().first() {
        sum += (byte as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}
