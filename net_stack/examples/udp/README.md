# UDP Echo Server & Client 示例

这个目录包含了基于 `net_stack` 网络协议栈实现的简单 UDP Server 和 UDP Client 示例。

## 1. 架构说明

这两个示例展示了如何在用户态应用程序中使用 `net_stack` 提供的 Socket API。

### 组件交互图

```mermaid
graph TD
    UserApp[用户应用 (Server/Client)] -->|bind/send_to/recv_from| UserSocket[UdpSocket (Wrapper)]
    UserSocket -->|操作| SocketSet[SocketSet (资源池)]
    SocketSet -->|包含| SocketState[UdpSocketState (内部状态)]
    
    subgraph "Net Stack (后台线程)"
        EventLoop[Event Loop] -->|poll| Rx[接收处理]
        EventLoop -->|poll| Tx[发送处理]
        Rx -->|分发| SocketState
        SocketState -->|数据入队| Tx
    end
```

*   **UDP Server**: 绑定到 `0.0.0.0:8080`，循环接收数据并将收到的内容原样发回（Echo）。
*   **UDP Client**: 绑定到随机端口（或指定端口），接收用户从标准输入的输入，发送给 Server，并等待回复。
*   **后台线程**: 两个示例都会启动一个后台线程运行 `net_stack::event_loop::run`，负责驱动整个协议栈的数据收发（ARP 自动解析、ICMP 响应、数据包分发等）。

## 2. 前置条件

在运行示例之前，请确保：

1.  **配置文件**: 项目根目录下存在 `net_stack.conf`，其中配置了本机的 IP 和 MAC 地址。
    ```ini
    # net_stack.conf 示例
    ip=192.168.1.100
    mac=aa:bb:cc:dd:ee:ff
    ```
2.  **权限**: 由于使用了 `pcap` 抓包和发包，运行程序通常需要 `sudo` 权限（或配置了相应的 capabilities）。
3.  **网络接口**: 确认你要使用的网络接口名称（如 `en0`, `eth0`）。

## 3. 编译与运行

假设你已经在 `Cargo.toml` 中配置了 `[[bin]]` 目标（或者作为 `examples` 运行）。

### 3.1 运行 UDP Server

在终端 1 中启动服务器：

```bash
# 替换 en0 为你的实际网卡名称
sudo cargo run --bin udp_server -- --config net_stack.conf --iface en0
```

成功启动后，你会看到：
```text
UDP Server listening on 0.0.0.0:8080
```

### 3.2 运行 UDP Client

在终端 2 中启动客户端：

```bash
# 替换 en0 为你的实际网卡名称
sudo cargo run --bin udp_client -- --config net_stack.conf --iface en0
```

成功启动后，你会看到提示输入消息。

## 4. 测试流程

1.  **启动 Server**: 按照上述步骤启动 Server。
2.  **启动 Client**: 按照上述步骤启动 Client。
3.  **发送消息**:
    *   在 Client 终端输入 `Hello World` 并回车。
    *   Client 会显示：`Received reply from 192.168.x.x:8080: Echo: Hello World`
    *   Server 终端会显示：`Received from 192.168.x.x:12345: Hello World`
4.  **跨机器测试** (可选):
    *   你可以在两台不同的机器上分别运行 Server 和 Client。
    *   确保两台机器在同一局域网。
    *   修改 `udp_client.rs` 中的 `target` IP 为 Server 机器的实际 IP。
    *   协议栈会自动处理 ARP 解析。

## 5. 常见问题

*   **Error: Device not found**: 检查 `--iface` 参数是否正确。
*   **收不到回复**:
    *   检查防火墙设置。
    *   检查 `net_stack.conf` 中的 IP/MAC 是否与本机实际网卡一致。
    *   如果是跨机器，确保 ARP 解析成功（可以通过抓包工具如 Wireshark 查看 ARP 交互）。
