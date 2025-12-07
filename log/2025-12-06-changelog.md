# 2025-12-06 Changelog

## 新增 ARP 发送能力
- `send/src/arp.rs` 新增 RFC 826 组帧逻辑，提供 `ArpOperation` 枚举与 `build_arp_payload` 帮助函数。
- CLI 增加 `--arp-mode`（Request/Reply）、`--arp-target-ip`、`--arp-target-mac` 选项，可直接从命令行构造 ARP 帧。
- `ethernet.rs` 根据模式自动选择 EtherType=0x0806，并对 ARP 载荷执行最小以太网长度填充，保持与 IPv4/原始载荷路径互斥。
- `mac.rs` 暴露 `from_raw` 与 `broadcast` 构造函数，便于在 ARP 场景中快速配置源/目的地址。

## 接收端 ARP 支持
- `receive/src/arp.rs` 新增解析与缓存组件，检测 EtherType=0x0806 时输出操作码、MAC/IP 等关键字段，并维护内存中的 ARP 表。
- `receive/src/ethernet.rs` 在抓包主循环中串联 `ArpProcessor`，并复用 IPv4 白名单(`--accept-ip`) 判定是否需要处理当前 ARP 报文。
- `ipv4.rs` 暴露 `allowed_slice`，方便 ARP 与 IPv4 共用相同的目标 IP 配置，保持与 C++ 版逻辑的一致性。
- `README.md` 更新，说明 ARP 报文在 send/receive 两侧的使用方式与调试要点。

## Mermaid 架构与流程图（ARP 版）

### 整体架构图
```mermaid
graph LR
	Workspace["ethernet_frame workspace"] --> SendCrate["send/ (Tx crate)"]
	Workspace --> RecvCrate["receive/ (Rx crate)"]

	subgraph SEND["send/ crate 模块"]
		direction TB
		SendCLI["cli.rsClap CLI"] --> SendETH["ethernet.rs帧拼装与 pcap"]
		SendETH --> SendCRC["crc.rsCRC32"]
		SendETH --> SendMAC["mac.rsMAC 工具"]
		SendETH --> SendIP["ipv4.rsIPv4 分片"]
		SendETH --> SendARP["arp.rsARP 组帧"]
	end

	subgraph RECV["receive/ crate 模块"]
		direction TB
		RecvCLI["cli.rsClap CLI"] --> RecvETH["ethernet.rspcap 捕获"]
		RecvETH --> RecvCRC["crc.rsCRC32"]
		RecvETH --> RecvMAC["mac.rsMAC 白名单"]
		RecvETH --> RecvIP["ipv4.rsIPv4 校验/重组"]
		RecvETH --> RecvARP["arp.rsARP 解析/缓存"]
	end

	Payload["data.txt / 自定义载荷"] --> SendETH
	SendETH --> Pcap["libpcap Active Capture"]
	Pcap --> RecvETH
	RecvETH --> RecvLog["recv.txt / payload log"]
	RecvIP --> IpOut["ip_output.bin"]
	RecvARP --> ArpCache["内存 ARP 缓存"]
```

### 运行流程图
```mermaid
flowchart TD
    start((程序启动))
    parse["Cli::parse()"]
    cmd{子命令}
    list_if["Device::list()"]
    done((结束))
    
    send_read["读取 data.txt"]
    mode{参数模式}
    build_arp["arp.rs 构造 28B 负载"]
    build_ip["ipv4.rs 分片"]
    enforce["检查长度/补零"]
    frames_send["ethernet.rs 拼装帧 + CRC32"]
    open_tx["打开接口"]
    loop_tx{达到 --count?}
    send_pkt["pcap.sendpacket"]
    wait{interval_ms>0?}
    sleep["thread::sleep"]
    
    open_rx["打开接口+输出文件"]
    capture["pcap.next_packet"]
    mac_ok{目的 MAC 白名单}
    crc_check["CRC32 校验"]
    ethertype{EtherType}
    ipv4_flow["ipv4.rs 校验/重组"]
    arp_flow["arp.rs 解析更新缓存"]
    log_payload["仅写 recv.txt"]
    limit_check{达到 --limit?}
    
    start --> parse
    parse --> cmd
    cmd -->|List| list_if
    list_if --> done
    
    cmd -->|Send| send_read
    send_read --> mode
    mode -->|ARP| build_arp
    mode -->|IPv4| build_ip
    mode -->|Raw| enforce
    build_arp --> frames_send
    build_ip --> frames_send
    enforce --> frames_send
    frames_send --> open_tx
    open_tx --> loop_tx
    loop_tx -->|否| send_pkt
    send_pkt --> wait
    wait -->|是| sleep
    sleep --> loop_tx
    wait -->|否| loop_tx
    loop_tx -->|是| done
    
    cmd -->|Receive| open_rx
    open_rx --> capture
    capture --> mac_ok
    mac_ok -->|否| capture
    mac_ok -->|是| crc_check
    crc_check --> ethertype
    ethertype -->|0x0800| ipv4_flow
    ethertype -->|0x0806| arp_flow
    ethertype -->|其他| log_payload
    ipv4_flow --> limit_check
    arp_flow --> limit_check
    log_payload --> limit_check
    limit_check -->|否| capture
    limit_check -->|是| done
```
