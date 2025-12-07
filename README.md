# Ethernet Frame 工具集使用指南

本项目包含两个二进制工具：

- `ethernet_frame_send`：从文件读取数据构造以太网帧，可选自动封装 IPv4 并发送。
- `ethernet_frame_receive`：基于 libpcap 捕获原始以太网帧，按 MAC/IP 白名单过滤后输出数据与 IPv4 重组结果。

下文说明如何编译、运行，以及本次新增的 IPv4 发送功能。

## 环境与依赖

- Rust 1.75+（推荐使用 `rustup`）
- 已安装 libpcap（macOS 自带，Linux 可通过包管理器安装 `libpcap-dev`）
- 运行发送/接收时通常需要管理员权限（如 `sudo`），因为需要打开原始网卡。

## 特性与设计

### Baremetal / No_std 兼容性
- **自定义 `Ipv4Addr` 实现**: 项目不依赖 `std::net::Ipv4Addr`，而是在 `ipv4_addr.rs` 中自行实现了完整的 IPv4 地址类型。
- **零标准库网络依赖**: 所有网络协议处理（以太网、IPv4、ARP、CRC32）均为自主实现，便于移植到嵌入式/裸金属环境。
- **完整 trait 支持**: 自定义类型实现了 `Display`, `FromStr`, `Debug`, `Eq`, `Ord`, `Hash` 等标准 trait，API 与标准库保持一致。

## 编译与发布

### 开发态调试

```bash
cargo run --bin ethernet_frame_send -- --help
cargo run --bin ethernet_frame_receive -- --help
```

### Release 版本

```bash
cargo build --workspace --release
```

生成的可执行文件位于 `target/release/`：

- `target/release/ethernet_frame_send`
- `target/release/ethernet_frame_receive`

运行示例（macOS）：

```bash
sudo ./target/release/ethernet_frame_send list
sudo ./target/release/ethernet_frame_receive list
```

## Send 工具使用

1. **列出接口**
   ```bash
   sudo ./target/release/ethernet_frame_send list
   ```
2. **发送基础以太网帧**
   ```bash
   sudo ./target/release/ethernet_frame_send \
     send --interface en0 --data data.txt --count 10 --interval-ms 500
   ```

常用参数：

- `--dest-mac/--src-mac`：源/目的 MAC 地址。
- `--ethertype`：自定义 EtherType（默认 `0x0080`）。
- `--pad`：若载荷 < 46 字节自动补零。
- `--count`：发送帧数，未指定则无限循环。
- `--interval-ms`：帧间隔，默认 1000ms。

## 新增：IPv4 封装与分片发送

启用 `--ipv4` 后，发送端会参考 WinPcap C++ 版本逻辑，自动构造 IPv4 头并完成必要分片：

```bash
sudo ./target/release/ethernet_frame_send send \
  --interface en0 \
  --data data.txt \
  --ipv4 \
  --src-ip 10.13.80.43 \
  --dst-ip 255.255.255.255 \
  --fragment-size 1400 \
  --protocol 6 \
  --ip-id 42
```

功能说明：

- `--ipv4`：开启 IPv4 封装后，EtherType 自动设置为 `0x0800`。
- `--src-ip/--dst-ip`：IP 头部地址字段；默认 `10.13.80.43 -> 255.255.255.255`。
- `--ttl`：生存时间；默认 64。
- `--tos`：服务类型；默认 `0xFE`，与原 C++ 保持一致。
- `--protocol`：上层协议号；默认 6 (TCP)。
- `--fragment-size`：单个片段的净载荷大小，需为 8 的倍数，默认 1400 字节（60 字节头 + 1400 数据 = 1460 以太网载荷）。
- `--ip-id`：IPv4 报文标识符，默认为 0，可自定义以便配合接收端调试重组逻辑。
- `--dont-fragment`：设置 DF 标志，禁止路由器再次分片。

发送端会先把输入文件切分为满足 `fragment_size` 的块，再为每个块生成 IPv4 头、计算校验和、填充标志/偏移，最后整体交由以太网层发送。循环发送时会按顺序轮询这些帧，从而重复播发完整的 IPv4 报文。

## 新增：ARP 帧构造与发送

`ethernet_frame_send` 现在可以直接构造 RFC 826 ARP 报文，无需准备数据文件：

```bash
sudo ./target/release/ethernet_frame_send send \
   --interface en0 \
   --arp-mode request \
   --src-mac 4a:c4:de:f0:3c:d8 \
   --src-ip 192.168.31.223 \
   --arp-target-ip 192.168.31.223 \
   --arp-target-mac 4a:c4:de:f0:3c:d8 \
   --count 10 --interval-ms 1000
```

关键开关：

- `--arp-mode <request|reply>`：选择发送 ARP 请求或响应；启用后 `--ipv4` 与自定义数据路径会自动关闭。
- `--arp-target-ip`：ARP 包中待解析/响应的 IPv4 地址，默认 `10.0.0.1`。
- `--arp-target-mac`：ARP 目标 MAC（请求时通常全 0，回复时为对端真实 MAC）。

开启 `--arp-mode` 后，程序会：

- 依据当前源 MAC/IP 与目标 MAC/IP 构造 28 字节 ARP 负载。
- 自动将 EtherType 设为 `0x0806`，并对 payload 进行 46 字节最小帧填充；无需再额外指定 `--ethertype` 或 `--pad`。
- 保持其他发包参数（`--count`、`--interval-ms`、`--dest-mac` 等）与普通模式一致，便于脚本化测试。

可配合 `ethernet_frame_receive` 观察 ARP 报文接收输出，或用 Wireshark 验证帧格式。

## Receive 工具使用

1. **列出接口**
   ```bash
   sudo ./target/release/ethernet_frame_receive list
   ```
2. **抓包并保存**
   ```bash
   sudo ./target/release/ethernet_frame_receive receive \
     --interface en0 \
     --output recv.txt \
     --ip-output ip_data.bin \
     --accept 4a:c4:de:f0:3c:d8,ff:ff:ff:ff:ff:ff \
     --accept-ip 192.168.31.223 \
     --limit 20
   ```

要点：

- 默认白名单包含广播 MAC 与示例目的 MAC，可用 `--accept` 增补。
- `--accept-ip` 控制 IPv4 目的地址白名单。
- 抓到 IPv4 分片时，接收端会重组后写入 `--ip-output` 指定文件，同时在终端打印 TTL、校验和、偏移等信息。

### 新增：ARP 解析与缓存

`ethernet_frame_receive` 会在检测到 EtherType `0x0806` 时解析 ARP 报文，并维护一个简单的 ARP 缓存表：

- 解析字段包含操作码、硬件/协议类型、发送端与目标 MAC/IP。
- 目标 IPv4 地址同样受 `--accept-ip` 白名单控制，可复用已有配置来表示“本机”地址。
- 每次学习到新的 IP↔MAC 对应关系都会在终端输出当前缓存，方便对照 C++ 版本的 `ARP_Cache_Table` 调试。

请确保以广播 MAC 或自定义白名单允许对应帧，否则以太网层会在进入 ARP 解析前被丢弃。

## 故障排查

- 权限不足：确保以 `sudo` 或具有 CAP_NET_RAW 的身份运行。
- 找不到接口：使用 `send list`/`receive list` 确认设备名（macOS 常为 `en0`）。
- IPv4 文件过大：当前实现以单个报文为单位，最大支持 `65535 - 60 = 65475` 字节净载荷，超出需自行拆分多次发送。

如需扩展更多协议或自动化测试，可在现有模块化结构基础上继续添加新的子模块或集成测试脚本。祝调试顺利！
