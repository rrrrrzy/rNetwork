# 2025-12-11 Changelog - 模块化重构与网络协议栈实现

## 重大架构变更：协议模块化 (Protocol Crate)

### 新增 `protocol` 独立模块
- **动机**: 消除 `send` 和 `receive` 模块之间的代码重复，建立统一的协议定义库，便于维护和扩展。
- **模块结构**:
  ```
  protocol/
  ├── src/
  │   ├── lib.rs          # 模块导出
  │   ├── arp.rs          # ARP 协议
  │   ├── checksum.rs     # CRC32 与简单校验和
  │   ├── ethernet.rs     # 以太网帧头
  │   ├── icmp.rs         # ICMP 协议
  │   ├── ipv4.rs         # IPv4 协议
  │   ├── mac.rs          # MAC 地址处理
  │   ├── udp.rs          # UDP 协议（预留）
  │   └── error.rs        # 统一错误类型
  ```

### 协议实现亮点
- **纯数据层设计**: 所有协议模块只负责数据结构的解析、序列化和验证，不包含 IO 操作。
- **零拷贝优化**: 使用 `&[u8]` 和固定大小数组，避免不必要的内存分配。
- **完整的验证机制**: 
  - IPv4: 校验和验证、版本检查、TTL 检查
  - ICMP: 类型/代码验证、时间戳合理性检查（可选）
  - ARP: 硬件类型、协议类型验证

### 协议模块迁移
- **移除重复代码**: 从 `send` 和 `receive` 模块中移除了以下文件：
  - `ipv4_addr.rs` (统一为 `protocol::ipv4::Ipv4Addr`)
  - `mac.rs` (统一为 `protocol::mac::MacAddr`)
  - `checksum.rs` (统一为 `protocol::checksum`)
  - `icmp.rs` (统一为 `protocol::icmp`)
  - 各自的 `error.rs` (合并到 `protocol::error`)

- **依赖更新**: `send` 和 `receive` 的 `Cargo.toml` 中添加:
  ```toml
  protocol = { path = "../protocol" }
  ```

## 新增 `net_stack` - 有状态网络协议栈

### 设计理念
`net_stack` 是一个**有状态**的网络协议栈实现，与 `send`/`receive` 这种无状态工具不同，它能够：
- 维护网络连接状态（ARP 表、Socket 表等）
- 主动发起网络请求（如 Ping）
- 自动响应网络请求（如 ARP 请求、ICMP Echo）

### 模块架构
```
net_stack/
├── src/
│   ├── main.rs           # 入口，管理 RX/TX 线程
│   ├── cli.rs            # 命令行参数解析
│   ├── stack.rs          # 协议栈核心，分发器
│   ├── handlers/         # 协议处理器
│   │   ├── mod.rs
│   │   ├── arp.rs        # ARP 请求/响应处理
│   │   ├── ipv4.rs       # IPv4 分发与发送
│   │   └── icmp.rs       # ICMP Echo 请求/响应
│   └── transport/        # 传输层（预留）
│       ├── mod.rs
│       ├── tcp.rs        # TCP 协议（待实现）
│       └── udp.rs        # UDP 协议（待实现）
```

### 核心功能

#### 1. 配置管理
- **配置文件支持**: 通过 `net_stack.conf` 指定本机 IP 和 MAC
  ```
  # net_stack.conf
  ip=192.168.31.223
  mac=4a:c4:de:f0:3c:d8
  ```
- **命令行参数**: 支持通过 `--ip` 和 `--mac` 直接指定
- **优先级**: 配置文件 > 命令行参数

#### 2. 被动响应能力
- **ARP 响应**: 
  - 自动响应针对本机 IP 的 ARP 请求
  - 将 (IP, MAC) 对加入 ARP 缓存（待完善）
  - 自动进行以太网帧填充（最小 60 字节）

- **ICMP Echo Reply**:
  - 接收到 ICMP Echo Request 后自动回复
  - 保持原始 ID、Seq、时间戳和载荷
  - 支持显示接收到的请求信息

#### 3. 主动发送能力
- **Ping 功能**: 通过 `--ping <IP>` 启动主动 Ping
  ```bash
  sudo ./target/release/net_stack \
    --config net_stack.conf \
    --iface en0 \
    --ping 192.168.31.55 \
    --target-mac a0:ad:9f:08:36:72
  ```
- **后台发送线程**: 独立线程每秒发送一次 ICMP Request
- **序列号管理**: 自动递增 Seq 字段
- **临时解决方案**: 需手动指定 `--target-mac`（完整 ARP 解析待实现）

### 技术实现

#### 分层架构
```
┌──────────────────────────────────────┐
│         main.rs (Event Loop)         │
│  RX Thread: pcap receive             │
│  TX Thread: periodic ping (optional)  │
└──────────────────────────────────────┘
                 ↓
┌──────────────────────────────────────┐
│      stack.rs (Protocol Stack)       │
│  - Ethernet frame parsing            │
│  - MAC filtering (self + broadcast)  │
│  - Protocol dispatcher               │
└──────────────────────────────────────┘
                 ↓
     ┌──────────┴──────────┐
     ↓                      ↓
┌──────────┐        ┌──────────────┐
│   ARP    │        │     IPv4     │
│ Handler  │        │   Handler    │
└──────────┘        └──────────────┘
                           ↓
                    ┌──────────────┐
                    │     ICMP     │
                    │   Handler    │
                    └──────────────┘
```

#### 关键设计决策
1. **线程安全**: 使用 `Arc<NetworkStack>` 在 RX 线程和 Ping 线程间共享
2. **互斥锁**: `sender: Arc<Mutex<Capture<Active>>>` 保护发送操作
3. **最小帧填充**: 所有发送路径（ARP、IPv4）都确保帧长度 ≥ 60 字节
4. **层次化发送**: ICMP → IPv4 → Ethernet，每层封装对应协议头

### 已知限制与未来计划
- [ ] **ARP 表**: 当前只打印 ARP Reply，未持久化到表中
- [ ] **自动 ARP 解析**: Ping 时需手动指定目标 MAC，应实现自动 ARP 查询
- [ ] **TCP/UDP**: 传输层模块已预留，待实现
- [ ] **多线程优化**: 考虑使用 tokio 异步运行时
- [ ] **错误处理**: 增强错误传播和恢复机制

## 代码质量改进

### 错误处理统一化
- 所有协议解析错误统一到 `protocol::error` 模块
- 使用 `Result<T, XxxParseError>` 替代 `panic!`
- 提供详细的错误信息（如校验和不匹配、长度错误等）

### 文档与注释
- 为所有公共 API 添加文档注释
- 关键算法（如校验和、分片）添加实现说明
- 版权声明与许可证信息（GPL-3.0）

### 测试覆盖
- 协议解析/序列化的往返测试
- 校验和计算正确性验证
- MAC/IPv4 地址解析测试

## 使用示例

### 1. 被动网络栈（仅响应）
```bash
sudo ./target/release/net_stack \
  --config net_stack.conf \
  --iface en0
```
功能：
- 响应 ARP 请求
- 回复 ICMP Echo Request

### 2. 主动 Ping
```bash
sudo ./target/release/net_stack \
  --config net_stack.conf \
  --iface en0 \
  --ping 192.168.31.55 \
  --target-mac a0:ad:9f:08:36:72
```
功能：
- 被动响应（同上）
- 每秒向 192.168.31.55 发送 ICMP Echo Request
- 自动接收并显示 ICMP Echo Reply

### 3. 使用命令行参数
```bash
sudo ./target/release/net_stack \
  --iface en0 \
  --ip 192.168.31.223 \
  --mac 4a:c4:de:f0:3c:d8 \
  --ping 8.8.8.8 \
  --target-mac <gateway-mac>
```

## 依赖更新
- `anyhow`: 错误处理
- `clap`: CLI 参数解析
- `pcap`: 网络抓包与发送
- `static_assertions`: 编译时断言

## 构建与测试
```bash
# 构建整个工作空间
cargo build --workspace --release

# 单独构建 net_stack
cargo build -p net_stack --release

# 运行测试
cargo test --workspace
```

## 已修复的 Bug
- **ARP 帧过短**: 添加填充到 60 字节，防止被网卡/交换机丢弃
- **IPv4 校验和**: 修正校验和计算时的字段设置顺序
- **ICMP 时间戳验证**: 放宽验证逻辑，兼容内核行为

## 性能与稳定性
- **零拷贝**: 协议解析尽可能使用引用，减少内存分配
- **超时处理**: pcap 设置 10ms 超时，避免阻塞主循环
- **错误恢复**: 单个包解析失败不影响后续包处理

## 文档更新
- 更新 `README.md`，增加 `net_stack` 使用说明
- 新增 `net_stack.conf` 配置文件模板
- 补充架构图和数据流图

## 致谢
> 致敬所有在深夜调试代码的灵魂。  
> 即便 Bug 如山，我亦往矣。

---

**版本**: v0.2.0-alpha  
**提交**: c8c1af0  
**日期**: 2025-12-11
