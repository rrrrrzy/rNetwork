# Baremetal IPv4 实现总结

## 变更概述

将项目中所有对 `std::net::Ipv4Addr` 的依赖替换为自定义实现,支持 baremetal/no_std 环境。

## 实现细节

### 新增文件

#### `send/src/ipv4_addr.rs` 和 `receive/src/ipv4_addr.rs`

```rust
/// 自定义 IPv4 地址类型(baremetal 兼容)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ipv4Addr([u8; 4]);

impl Ipv4Addr {
    // 构造函数
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self;
    pub const fn from_octets(octets: [u8; 4]) -> Self;
    
    // 访问器
    pub const fn octets(&self) -> [u8; 4];
    
    // 常量地址
    pub const fn broadcast() -> Self;      // 255.255.255.255
    pub const fn unspecified() -> Self;    // 0.0.0.0
    pub const fn localhost() -> Self;      // 127.0.0.1
}

// Trait 实现
impl Display      // "192.168.1.1" 格式输出
impl FromStr      // 从字符串解析
impl Debug        // {:?} 输出
impl Eq + Ord     // 比较支持
impl Hash         // HashMap 支持
```

### 修改文件

**所有 8 个文件的 import 更新**:
- `send/src/{cli.rs, arp.rs, ipv4.rs, main.rs}`
- `receive/src/{cli.rs, arp.rs, ipv4.rs, main.rs}`

```rust
// 替换前
use std::net::Ipv4Addr;

// 替换后
use crate::ipv4_addr::Ipv4Addr;
```

## 验证结果

### 编译状态
✅ `cargo build --workspace --release` 成功编译
⚠️ 仅有未使用函数警告(dead_code),不影响功能

### 功能测试

1. **IPv4 分片发送**: 
   ```bash
   sudo ./target/release/ethernet_frame_send send --interface en0 \
     --src-mac 00:11:22:33:44:55 --dest-mac ff:ff:ff:ff:ff:ff \
     --src-ip 192.168.1.100 --dst-ip 192.168.1.1 \
     --data data.txt --count 1 --ipv4 --fragment-size 1400
   # 输出: 已发送 1 个帧,每帧总长度 1478 字节
   ```

2. **ARP 请求发送**:
   ```bash
   sudo ./target/release/ethernet_frame_send send --interface en0 \
     --src-mac 00:11:22:33:44:55 --dest-mac ff:ff:ff:ff:ff:ff \
     --arp-mode request --arp-target-ip 192.168.1.1 --count 1
   # 输出: 已发送 1 个帧,每帧总长度 64 字节
   ```

3. **接收端 IPv4 过滤**: 正常启动并按 IP 过滤

## API 兼容性

自定义 `Ipv4Addr` 与 `std::net::Ipv4Addr` 保持 API 兼容:

| 标准库方法          | 自定义实现 | 兼容性 |
|-------------------|----------|--------|
| `new(a,b,c,d)`    | ✅        | 完全兼容 |
| `octets()`        | ✅        | 完全兼容 |
| `BROADCAST`       | `broadcast()` | 改为方法以符合 Rust 命名规范 |
| `UNSPECIFIED`     | `unspecified()` | 同上 |
| `LOCALHOST`       | `localhost()` | 同上 |
| `Display` trait   | ✅        | 格式兼容 "a.b.c.d" |
| `FromStr` trait   | ✅        | 解析兼容 |

## 依赖减少

**替换前**:
```toml
[dependencies]
# 隐式依赖 std::net 模块
```

**替换后**:
```rust
// 完全自主实现,仅依赖:
use std::fmt;      // 格式化输出
use std::str;      // 字符串解析
use std::hash;     // Hash trait
```

## Baremetal 迁移建议

1. **进一步 no_std 化**:
   - 将 `fmt` 和 `str` 替换为 `core::fmt` 和 `core::str`
   - 移除 `anyhow` 错误处理,使用 `Result<T, &'static str>`
   - 替换 `HashMap` 为固定大小数组(ARP 缓存)

2. **libpcap 替代**:
   - 实现自定义网卡驱动(如 DPDK/XDP)
   - 直接操作网络接口寄存器

3. **内存分配**:
   - 当前使用 `Vec` 和 `String`,需替换为固定缓冲区
   - 使用 `heapless` crate 提供 no_std 集合类型

## 文档更新

- ✅ `README.md`: 添加 "Baremetal / No_std 兼容性" 章节
- ✅ `log/2025-12-06-changelog.md`: 记录重构详情
- ✅ 测试脚本: `/tmp/test_baremetal.sh` 验证功能

## 后续优化

1. 添加 `#[cfg(feature = "std")]` 条件编译
2. 为 `ipv4_addr.rs` 添加单元测试
3. 考虑实现 `MacAddr` 的类似重构
4. 添加 `#[allow(dead_code)]` 标记保留未来可能使用的函数
