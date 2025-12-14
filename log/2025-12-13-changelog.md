# 2025-12-13 Changelog - UDP 传输层与示例

## Added
- **UDP 传输层可用**：引入 `UdpSocketState`（收发队列）+ `UdpSocket` Wrapper，提供 `bind / send_to / recv_from` API，使用五元组 `SocketHandle` 与 `SocketSet` 的多级匹配（精确/指定 IP/通配）。
- **示例应用**：新增 `udp_server` 与 `udp_client` 两个二进制目标（Echo Server/Client），支持从 CLI 直接运行，示范 Socket API 与后台事件循环协同工作。
- **库入口**：`net_stack/src/lib.rs` 暴露核心模块，支持以库形式被示例和外部代码引用。
- **ARP 自动解析链路**：事件循环中在发送路径发起 ARP 请求，并在收到 ARP 响应后批量发送挂起数据包。

## Changed
- **Cargo 配置**：`net_stack/Cargo.toml` 增加 `[[bin]]` 目标，方便 `cargo run --bin udp_server` / `udp_client` 直接启动。
- **README 文档**：补充 UDP 示例的运行方式、前置条件与测试流程。

## Fixed
- 修复 IPv4 调用链中对 `stack` 的句柄传递错误，确保分发器调用正确的处理函数。

## Notes
- 当前 UDP 功能为基础版本（无重传、无端口不可达 ICMP 回复），但已支持基础的收发和 Echo 验证。
- ARP 表已具备自动学习与发送挂起包的能力，但持久化与老化策略后续再完善。

## Architecture (2025-12-13)
架构图
```mermaid
graph TD
    %% 设置节点样式为左对齐，背景色浅灰，边框深灰，类似截图风格
    classDef plain fill:#f5f5f5,stroke:#333,stroke-width:1px,text-align:left;

    %% 1. 定义节点
    %% 使用 <br> 进行换行
    MainNode["<b>Event Loop</b><br>| RX Thread: pcap receive<br>| TX Thread: periodic ping (optional)"]
    
    StackNode["<b>stack.rs (Protocol Stack)</b><br>- Ethernet frame parsing<br>- MAC filtering (self + broadcast)<br>- Protocol dispatcher"]
    
    ARPNode["<b>ARP</b><br>Handler"]
    ARPTableNode["<b>ARPTable</b><br>Handler"]
    IPv4Node["<b>IPv4</b><br>Handler"]
    UDPHNode["<b>UDP</b><br>Handler"]
    ICMPNode["<b>ICMP</b><br>Handler"]
    SocketNode["<b>SocketSet</b><br>Transport"]
    UdpSocketAPI["<center><b>UdpSocket API</b></center>Transport<br><em>bind, sendto, recv_from</em>"]

    %% 2. 定义连接关系
    MainNode --> StackNode
    
    %% 这里表示从 StackNode 分叉出两个箭头
    StackNode --> ARPNode
    StackNode --> IPv4Node
    StackNode --> SocketNode
    
    ARPNode  --> ARPTableNode
    IPv4Node --> ICMPNode
    IPv4Node --> UDPHNode
    SocketNode --> UdpSocketAPI

    %% 3. 应用样式
    class MainNode,StackNode,ARPNode,IPv4Node,ARPTableNode,ICMPNode,UDPHNode,SocketNode,UdpSocketAPI plain
```

流程图

```mermaid
graph TD
	classDef box fill:#f5f5f5,stroke:#333,stroke-width:1px,text-align:left;

	Main["net_stack main (binary)<br>• 解析 CLI/配置<br>• 启动 event_loop"]
	Loop["event_loop::run<br>• pcap RX<br>• poll_and_send (TX)<br>• ARP 挂起队列调度"]
	Stack["stack.rs<br>• 以太网解析/分发<br>• MAC 过滤<br>• 调用 handlers"]

	ARP["handlers::arp<br>• ARP 请求/响应<br>• 学习 ARP 表<br>• 唤醒挂起包"]
	IPv4["handlers::ipv4<br>• IPv4 解析/封装<br>• 分发 ICMP/UDP"]
	ICMP["handlers::icmp<br>• Echo Request/Reply"]
	UDPH["handlers::udp<br>• UDP 解析/校验<br>• socket_set.lookup -> rx_enqueue"]

	Sockets["SocketSet (HashMap<SocketHandle, Socket>)<br>• 五元组匹配<br>• 精确/指定IP/通配"]
	UdpState["UdpSocketState<br>• rx/tx 队列<br>• bind/send_to/recv_from"]

	Main --> Loop --> Stack
	Stack --> ARP
	Stack --> IPv4
	IPv4 --> ICMP
	IPv4 --> UDPH
	UDPH --> Sockets
	Sockets --> UdpState

	class Main,Loop,Stack,ARP,IPv4,ICMP,UDPH,Sockets,UdpState box
```
