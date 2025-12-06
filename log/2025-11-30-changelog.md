# 2025-11-30 Changelog

## Commit 2105b6f564d9e15371d003467db19d943bda4d9b
- 拆分原单体 `src/main.rs`，建立 `send/` 与 `receive/` 两个独立二进制入口，并创建对应的 `Cargo.toml`。
- 新增发送端模块化结构（`cli.rs/crc.rs/ethernet.rs/ipv4.rs/mac.rs`），实现 CLI、CRC32、IPv4 分片构造与帧发送管线。
- 新增接收端模块化结构（`cli.rs/crc.rs/ethernet.rs/ipv4.rs/mac.rs`），为后续 IPv4 校验与分片重组打下基础。
- 更新 `recv.txt`，提供示例抓包输出。

## 后续工作（基于最近的协作需求）
- **IPv4 发送增强**：`ethernet_frame_send` 增加 `--ipv4` 相关选项，支持指定源/目的 IP、TTL、TOS、协议号、分片大小、DF 标志及报文 ID；在发送循环中轮播所有分片，对应接收端重组逻辑。
- **接收端 UTF-8 适配**：`receive/src/ethernet.rs` 在打印帧内容时先识别 IPv4，有效地跳过 IP 头再尝试 UTF-8 解码，不可解码时回退到 ASCII/十六进制转储。
- **分片重组日志**：`receive/src/ipv4.rs` 在无分片或重组完成后，都会再次输出完整的 IP 负载（优先 UTF-8），方便对照 `ip_output` 文件内容。
- **使用文档**：新增 `README.md`，覆盖开发/Release 构建流程、send/receive 命令示例及 IPv4 新功能说明。

## Mermaid 架构与流程图

### 整体架构图
```mermaid
graph LR
    Workspace["ethernet_frame workspace"] --> SendCrate["send/ (Tx crate)"]
    Workspace --> RecvCrate["receive/ (Rx crate)"]

    subgraph SEND["send/ crate 模块"]
        direction TB
        SendCLI["cli.rsClap CLI 解析"] --> SendETH["ethernet.rs帧拼装与 pcap"]
        SendETH --> SendCRC["crc.rsCRC32 查表"]
        SendETH --> SendMAC["mac.rsMAC 解析"]
        SendETH --> SendIP["ipv4.rsIPv4 报文与分片"]
    end

    subgraph RECV["receive/ crate 模块"]
        direction TB
        RecvCLI["cli.rsClap CLI 解析"] --> RecvETH["ethernet.rspcap 捕获与过滤"]
        RecvETH --> RecvCRC["crc.rsCRC32 校验"]
        RecvETH --> RecvMAC["mac.rsMAC 白名单"]
        RecvETH --> RecvIP["ipv4.rsIPv4 校验与重组"]
    end

    Payload["data.txt / 自定义载荷"] --> SendETH
    SendETH --> Pcap["libpcap Active Capture"]
    Pcap --> RecvETH
    RecvETH --> RecvLog["recv.txt / payload log"]
    RecvIP --> IpOut["ip_output.bin (重组结果)"]
```

### 运行流程图
```mermaid
flowchart TD
    start((程序启动))
    parse["Cli::parse()"]
    cmd{选择的子命令}
    list["Device::list() 列出可用接口"]
    done((结束))
    
    start --> parse
    parse --> cmd
    cmd -->|List| list
    list --> done

    send_read[读取 data.txt 载荷]
    send_ipv4{--ipv4 选项?}
    build_ip[ipv4.rs 构造报文并分片]
    enforce[检查长短并按需补零]
    frames[ethernet.rs 拼装帧 + CRC32]
    open_tx[打开指定网卡并进入发送循环]
    send_loop{达到 --count?}
    tx[pcap sendpacket 发送一帧]
    wait{interval_ms > 0?}
    sleep[thread::sleep]
    
    cmd -->|Send| send_read
    send_read --> send_ipv4
    send_ipv4 -->|是| build_ip
    send_ipv4 -->|否| enforce
    build_ip --> frames
    enforce --> frames
    frames --> open_tx
    open_tx --> send_loop
    send_loop -->|否| tx
    tx --> wait
    wait -->|是| sleep
    wait -->|否| send_loop
    sleep --> send_loop
    send_loop -->|是| done

    open_rx[打开网卡 + 输出文件]
    capture[pcap.next_packet 阻塞抓包]
    valid_len{">= 14+CRC?"}
    mac_filter[目的 MAC 白名单]
    crc_check[CRC32 校验与日志]
    ethertype{"EtherType == 0x0800?"}
    ipv4_flow[ipv4.rs 校验 + 重组 + 写入 ip_output]
    log_only[记录载荷并刷新 recv.txt]
    limit_check{达到 --limit?}
    
    cmd -->|Receive| open_rx
    open_rx --> capture
    capture --> valid_len
    valid_len -->|否| capture
    valid_len -->|是| mac_filter
    mac_filter -->|否| capture
    mac_filter -->|是| crc_check
    crc_check --> ethertype
    ethertype -->|是| ipv4_flow
    ethertype -->|否| log_only
    ipv4_flow --> limit_check
    log_only --> limit_check
    limit_check -->|否| capture
    limit_check -->|是| done
```
