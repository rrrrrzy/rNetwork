# 以太网帧练习工具

这是原 WinPcap 版发送端与接收端的 Rust 实现。程序依赖 `libpcap`（通过 `pcap` crate）来构造以太网帧、在指定接口上发送数据，并捕获目标 MAC 匹配的帧。

## 环境要求

- Rust 工具链（建议 1.75 及以上）
- 支持 libpcap 的系统（macOS 自带，Linux 通常需要安装 `libpcap-dev`）
- 发送/抓取原始帧往往需要管理员权限，若遇到权限错误可使用 `sudo`

## 构建

```bash
cargo build
```

## 使用示例

列出所有 libpcap 可用接口：

```bash
cargo run -- list
```

在接口 `en0` 上发送 `data.txt` 的内容（一次发送 10 个帧、间隔 500ms）：

```bash
cargo run -- send --interface en0 --data data.txt --count 10 --interval-ms 500
```

在 `en0` 上抓包，并把载荷保存到 `recv.txt`，最多捕获 5 个匹配帧：

```bash
cargo run -- receive --interface en0 --output recv.txt --limit 5
```

还可以通过其他参数修改 MAC 地址、为短数据补零、调节发送速率，或扩展允许的目的 MAC 白名单。更多选项可运行 `cargo run -- --help`，或查看 `send`/`receive` 子命令的帮助信息。
