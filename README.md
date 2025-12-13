[developping]
项目仍在开发中，请谨慎使用

# Ethernet Frame 工具集使用指南

本项目是一个基于 Rust 的**网络协议栈实现与调试工具集**，包含三个核心模块：

1. **`protocol`** - 独立的协议库（ARP、IPv4、ICMP、以太网等）
2. **`send`/`receive`** - 无状态的发送/接收工具（用于测试）
3. **`net_stack`** - 有状态的网络协议栈（支持主动/被动通信）

## 快速开始

### 环境要求
- **Rust**: 1.75+ (推荐使用 `rustup`)
- **libpcap**: macOS 自带；Linux 需安装 `libpcap-dev`
- **权限**: 需要 `sudo` 或 `CAP_NET_RAW` 能力来访问网卡

### 编译
```bash
# 编译整个工作空间
cargo build --workspace --release

# 可执行文件位于 target/release/
# - ethernet_frame_send
# - ethernet_frame_receive
# - net_stack
# - udp_server        # UDP Echo Server 示例
# - udp_client        # UDP Echo Client 示例
```

## 核心特性

### 🎯 Baremetal / No_std 兼容
- **零标准库依赖**: 所有协议实现不依赖 `std::net`，可移植到嵌入式环境
- **自定义类型**: 实现了 `Ipv4Addr`、`MacAddr` 等类型，完全兼容 std API
- **纯 Rust 实现**: CRC32、IPv4 校验和、ARP、ICMP 等协议从零实现

### 📦 模块化架构
```
protocol/          # 协议定义库（纯数据层）
├── arp.rs         # ARP 协议
├── ethernet.rs    # 以太网帧
├── ipv4.rs        # IPv4 协议
├── icmp.rs        # ICMP 协议
├── mac.rs         # MAC 地址
├── checksum.rs    # CRC32 & 简单校验和
└── error.rs       # 统一错误类型

send/              # 发送工具（无状态）
receive/           # 接收工具（无状态）
net_stack/         # 网络协议栈（有状态）
```

---

## 一、`net_stack` - 网络协议栈

`net_stack` 是一个**有状态**的网络协议栈实现，可以：
- 被动响应 ARP 请求和 ICMP Echo Request
- 主动发起 ICMP Ping 请求
- 维护网络状态（ARP 表、连接表等）

### 配置

#### 方式 1: 使用配置文件（推荐）
创建 `net_stack.conf`:
```ini
# 本机 IP 地址
ip=192.168.31.223

# 本机 MAC 地址
mac=4a:c4:de:f0:3c:d8
```

#### 方式 2: 命令行参数
```bash
sudo ./target/release/net_stack \
  --iface en0 \
  --ip 192.168.31.223 \
  --mac 4a:c4:de:f0:3c:d8
```

### 使用场景

#### 场景 1: 被动网络栈（响应模式）
```bash
sudo ./target/release/net_stack --config net_stack.conf --iface en0
```

**功能**：
- ✅ 自动响应 ARP 请求（Who has X.X.X.X? Tell Y.Y.Y.Y）
- ✅ 自动回复 ICMP Echo Request（Ping）
- ✅ 打印接收到的网络活动

**输出示例**：
```
Starting Network Stack on interface: en0
Stack initialized. IP: 192.168.31.223, MAC: 4a:c4:de:f0:3c:d8
Waiting for packets...
收到 ARP 请求: 谁是 192.168.31.223? (来自 192.168.31.1)
Received ICMP Request from 192.168.31.55
```

#### 场景 2: 主动 Ping
```bash
sudo ./target/release/net_stack \
  --config net_stack.conf \
  --iface en0 \
  --ping 192.168.31.55 \
  --target-mac a0:ad:9f:08:36:72
```

**功能**：
- ✅ 被动响应（同上）
- ✅ 每秒向目标 IP 发送 ICMP Echo Request
- ✅ 自动接收并显示 ICMP Echo Reply

**参数说明**：
- `--ping <IP>`: 目标 IP 地址
- `--target-mac <MAC>`: 目标 MAC 地址（临时方案，未来将通过 ARP 自动解析）

**输出示例**：
```
Starting Ping to 192.168.31.55 (MAC: a0:ad:9f:08:36:72)
Sending ICMP Request seq=1 to 192.168.31.55
Sending ICMP Request seq=2 to 192.168.31.55
Received ICMP Reply from 192.168.31.55
```

#### 场景 3: UDP Echo 示例
```bash
# 启动 Server
sudo cargo run --bin udp_server -- --config net_stack.conf --iface en0

# 启动 Client（另一个终端）
sudo cargo run --bin udp_client -- --config net_stack.conf --iface en0
```

**说明**：
- Server 绑定在 `0.0.0.0:8080`，收到消息后原样回显。
- Client 默认绑定 `0.0.0.0:12345`，从 stdin 读取消息，发送到 Server 并等待回复。
- 如果跨机器测试，请修改 Client 代码中的目标 IP 为 Server 所在主机的 IP。

### 架构设计

```
┌─────────────────────────────────────────┐
│  main.rs (Event Loop)                   │
│  • RX Thread: 持续接收网络包            │
│  • TX Thread: 定时发送（可选）          │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  stack.rs (Protocol Dispatcher)         │
│  • 以太网帧解析                         │
│  • MAC 地址过滤 (self + broadcast)     │
│  • 协议分发 (ARP / IPv4)                │
└─────────────────────────────────────────┘
          ↓                    ↓
┌──────────────┐      ┌──────────────────┐
│ handlers/arp │      │ handlers/ipv4    │
│  • Request   │      │  • Protocol 分发 │
│  • Reply     │      │  • 封装与发送    │
└──────────────┘      └──────────────────┘
                              ↓
                      ┌───────────────┐
                      │ handlers/icmp │
                      │  • Echo Req   │
                      │  • Echo Reply │
                      └───────────────┘
```

### 已实现功能
- ✅ ARP 请求/响应 + 自动学习并驱动挂起包发送
- ✅ ICMP Echo Request/Reply (Ping)
- ✅ IPv4 分发与封装（自动填充到最小 60 字节）
- ✅ UDP Socket（bind / send_to / recv_from，基础队列转发）
- ✅ 配置文件支持（IP/MAC）

### 待实现功能
- ⏳ ARP 表持久化/老化策略
- ⏳ UDP 增强（端口不可达 ICMP、并发调度等）
- ⏳ TCP 协议支持（三次握手、可靠传输）
- ⏳ Socket 接口高级特性（非阻塞/超时等）

---

## 二、`send` / `receive` 工具

这两个工具是**无状态**的网络调试工具，用于发送和接收原始以太网帧。

### Send 工具

#### 1. 列出网络接口
```bash
sudo ./target/release/ethernet_frame_send list
```

#### 2. 发送基础以太网帧
```bash
sudo ./target/release/ethernet_frame_send send \
  --interface en0 \
  --data data.txt \
  --dest-mac 44:87:fc:d6:bd:8c \
  --src-mac 44:87:fc:d6:bf:91 \
  --count 10 \
  --interval-ms 500
```

**常用参数**:
- `--dest-mac` / `--src-mac`: 源/目的 MAC 地址
- `--ethertype`: 自定义 EtherType（默认 `0x0080`）
- `--pad`: 若载荷 < 46 字节自动补零
- `--count`: 发送帧数（未指定则无限循环）
- `--interval-ms`: 帧间隔（默认 1000ms）

#### 3. IPv4 封装与分片发送
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

**IPv4 参数**:
- `--ipv4`: 启用 IPv4 封装（EtherType 自动设为 `0x0800`）
- `--src-ip` / `--dst-ip`: IP 地址
- `--ttl`: 生存时间（默认 64）
- `--tos`: 服务类型（默认 `0xFE`）
- `--protocol`: 上层协议号（默认 6 = TCP）
- `--fragment-size`: 单片净载荷大小，需为 8 的倍数（默认 1400）
- `--ip-id`: 报文标识符（默认 0）
- `--dont-fragment`: 设置 DF 标志

#### 4. ARP 帧构造与发送
```bash
sudo ./target/release/ethernet_frame_send send \
  --interface en0 \
  --arp-mode request \
  --src-mac 4a:c4:de:f0:3c:d8 \
  --src-ip 192.168.31.223 \
  --arp-target-ip 192.168.31.1 \
  --arp-target-mac 00:00:00:00:00:00 \
  --count 10
```

**ARP 参数**:
- `--arp-mode <request|reply>`: ARP 操作类型
- `--arp-target-ip`: 目标 IP 地址
- `--arp-target-mac`: 目标 MAC 地址（请求时通常全 0）
- 启用 ARP 后 EtherType 自动设为 `0x0806`

### Receive 工具

#### 1. 列出网络接口
```bash
sudo ./target/release/ethernet_frame_receive list
```

#### 2. 抓包并保存
```bash
sudo ./target/release/ethernet_frame_receive receive \
  --interface en0 \
  --output recv.txt \
  --ip-output ip_data.bin \
  --accept 4a:c4:de:f0:3c:d8,ff:ff:ff:ff:ff:ff \
  --accept-ip 192.168.31.223 \
  --limit 20
```

**参数说明**:
- `--accept`: MAC 地址白名单（逗号分隔）
- `--accept-ip`: IPv4 地址白名单
- `--output`: 以太网载荷输出文件
- `--ip-output`: IPv4 重组后数据输出文件
- `--limit`: 抓取包数量限制

**功能**:
- ✅ ARP 报文解析与缓存
- ✅ IPv4 分片重组
- ✅ ICMP 报文解析
- ✅ MAC/IP 白名单过滤

---

## 三、`protocol` 库

`protocol` 是一个独立的协议库，提供零依赖的网络协议实现。

### 主要类型

#### MacAddr
```rust
use protocol::mac::MacAddr;

let mac = MacAddr::from_raw([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
let broadcast = MacAddr::broadcast();
println!("{}", mac); // 11:22:33:44:55:66
```

#### Ipv4Addr
```rust
use protocol::ipv4::Ipv4Addr;
use std::str::FromStr;

let ip = Ipv4Addr::new(192, 168, 1, 1);
let ip2 = Ipv4Addr::from_str("192.168.1.1").unwrap();
assert_eq!(ip, ip2);
```

#### Ipv4Header
```rust
use protocol::ipv4::{Ipv4Header, Ipv4Addr};

let header = Ipv4Header::new(
    Ipv4Addr::new(192, 168, 1, 1),  // src
    Ipv4Addr::new(192, 168, 1, 2),  // dst
    1,      // protocol (ICMP)
    64,     // payload_len
    12345   // id
);

let bytes = header.to_bytes();
assert!(header.validate().is_ok());
```

#### ICMP
```rust
use protocol::icmp::{ICMP, IcmpType};

let ping = ICMP::new(
    IcmpType::Request,
    0,      // code
    1234,   // id
    1,      // seq
    timestamp,
    &payload_data
);

let bytes = ping.to_bytes();
let parsed = ICMP::parse(&bytes)?;
```

#### ARP
```rust
use protocol::arp::{ArpPacket, ArpOperation};

let arp = ArpPacket::new(
    ArpOperation::Request,
    sender_mac,
    sender_ip,
    target_mac,
    target_ip
);

let bytes = arp.to_bytes();
```

### 校验和函数
```rust
use protocol::checksum::{simple_checksum, Crc32};

// IPv4/ICMP 简单校验和
let checksum = simple_checksum(&header_bytes);

// CRC32
let crc = Crc32::new();
let crc_value = crc.checksum(&data);
```

---

## 故障排查

### 1. 权限不足
```bash
Error: Permission denied
```
**解决**: 使用 `sudo` 运行或赋予 `CAP_NET_RAW` 权限

### 2. 找不到网络接口
```bash
Error: Device not found: en0
```
**解决**: 使用 `list` 命令查看可用接口
```bash
sudo ./target/release/net_stack list  # 如果支持
# 或者
ifconfig  # macOS/Linux
ip link   # Linux
```

### 3. 收不到 ICMP 回包
**可能原因**:
1. 目标主机未响应（检查 `ping` 命令是否能通）
2. 目标 MAC 地址错误（需要与目标 IP 在同一网段的正确 MAC）
3. 防火墙阻止（检查本地和远程防火墙）
4. 帧长度不足（已修复：自动填充到 60 字节）

**调试步骤**:
```bash
# 1. 用系统 ping 测试连通性
ping 192.168.31.55

# 2. 查看 ARP 表确认 MAC 地址
arp -a | grep 192.168.31.55

# 3. 使用 tcpdump 抓包验证
sudo tcpdump -i en0 -vv icmp

# 4. 检查网卡是否处于混杂模式
ifconfig en0 | grep PROMISC
```

### 4. ARP 请求无响应
**检查**:
- 配置的 IP/MAC 是否正确
- 网卡是否在正确的网段
- 是否有其他程序占用了网卡

---

## 开发与调试

### 运行测试
```bash
cargo test --workspace
```

### 开发模式编译
```bash
cargo build --workspace
```

### 查看详细日志
在代码中添加更多 `println!` 或使用 `env_logger`:
```rust
env_logger::init();
log::debug!("Received packet: {:?}", packet);
```

### Wireshark 抓包
配合 Wireshark 可以验证发送的帧格式：
```bash
# 在一个终端运行程序
sudo ./target/release/net_stack --config net_stack.conf --iface en0 --ping 192.168.31.55 --target-mac xx:xx:xx:xx:xx:xx

# 在另一个终端抓包
sudo tcpdump -i en0 -w capture.pcap

# 用 Wireshark 打开 capture.pcap 分析
```

---

## 贡献与反馈

### 项目结构
```
.
├── protocol/           # 协议库
├── send/               # 发送工具
├── receive/            # 接收工具
├── net_stack/          # 网络协议栈
├── log/                # 变更日志
├── Cargo.toml          # 工作空间配置
└── README.md           # 本文件
```

### 开发路线图
- [x] 基础以太网帧收发
- [x] IPv4 分片与重组
- [x] ARP 协议支持
- [x] ICMP Echo 支持
- [x] 模块化重构（protocol crate）
- [x] 有状态协议栈（net_stack）
- [ ] UDP 协议支持
- [ ] TCP 协议支持
- [ ] Socket 接口抽象
- [ ] 完整的 ARP 表管理
- [ ] 多线程性能优化

### 变更日志
详见 `log/` 目录:
- [2025-11-30-changelog.md](log/2025-11-30-changelog.md)
- [2025-12-06-changelog.md](log/2025-12-06-changelog.md)
- [2025-12-11-changelog.md](log/2025-12-11-changelog.md)

### 许可证
本项目采用 **GPL-3.0** 许可证。

> 致敬所有在深夜调试代码的灵魂。  
> 即便 Bug 如山，我亦往矣。

---

**最后更新**: 2025-12-113 
**版本**: v0.2.1-alpha  
**提交**: 35dac02
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
