# mcp2cli-rs

[mcp2cli](https://github.com/knowsuchagency/mcp2cli) 的 Rust 语言实现版本。

一个极简、无状态的命令行工具，用于将 MCP（Model Context Protocol）服务器和 OpenAPI 规范转换为可执行的 CLI 命令。

## 特性

- **轻量级**：单一二进制文件，体积小于 3MB
- **无状态**：无缓存、无会话、无持久化存储
- **机器优先**：JSON 格式输出，便于程序解析
- **零依赖**：静态链接，无需运行时库

## 安装

### 预编译二进制文件（推荐）

从 [GitHub Releases](https://github.com/jshachm/mcp2cli-rs/releases) 下载适合您平台的预编译二进制文件：

| 平台 | 架构 | 下载链接 |
|------|------|----------|
| Linux | x86_64 | `mcp2cli-rs-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `mcp2cli-rs-aarch64-unknown-linux-musl.tar.gz` |
| macOS | x86_64 | `mcp2cli-rs-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `mcp2cli-rs-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `mcp2cli-rs-x86_64-pc-windows-gnu.zip` |

**Linux 安装示例：**
```bash
# 下载并解压
curl -LO https://github.com/jshachm/mcp2cli-rs/releases/latest/download/mcp2cli-rs-x86_64-unknown-linux-musl.tar.gz
tar xzf mcp2cli-rs-x86_64-unknown-linux-musl.tar.gz

# 移动到 PATH
sudo mv mcp2cli-rs /usr/local/bin/
sudo chmod +x /usr/local/bin/mcp2cli-rs
```

### 从源码构建

```bash
git clone git@github.com:jshachm/mcp2cli-rs.git
cd mcp2cli-rs
cargo build --release
# 二进制文件位置: target/release/mcp2cli-rs
```

### Docker 运行

```bash
docker run --rm ghcr.io/jshachm/mcp2cli-rs:latest --help
```

## 使用方法

### MCP HTTP 模式

通过 HTTP/SSE 协议连接远程 MCP 服务器：

```bash
# 列出可用工具
mcp2cli-rs mcp https://api.example.com/mcp --auth-header "Authorization:Bearer <token>" --list --json

# 调用工具
mcp2cli-rs mcp https://api.example.com/mcp --auth-header "Authorization:Bearer <token>" -- \
  tool_name --param1 value1 --param2 value2 --json
```

### MCP stdio 模式

通过标准输入输出连接本地 MCP 服务器：

```bash
# 列出可用工具
mcp2cli-rs mcp-stdio --list "/opt/homebrew/bin/mcp-server-filesystem" "/Users/workspace"

# 调用工具
mcp2cli-rs mcp-stdio "/opt/homebrew/bin/mcp-server-filesystem" "/Users/workspace" -- \
  read_file --path "/Users/workspace/test.txt"
```

### OpenAPI 模式

将 OpenAPI 规范转换为 CLI 命令：

```bash
# 列出所有端点
mcp2cli-rs spec ./openapi.json --base-url https://api.example.com --list --json

# 调用端点
mcp2cli-rs spec ./openapi.json --base-url https://api.example.com -- \
  operation_id --param value --json
```

## CLI 参数

### 全局参数

- `--json`: 以 JSON 格式输出（机器可读）
- `--timeout <秒>`: 请求超时时间，默认 30 秒

### MCP HTTP 参数

- `--auth-header <Name:Value>`: 自定义 HTTP 请求头（可多次使用）
- `--list`: 列出可用工具

### MCP stdio 参数

- `--list`: 列出可用工具
- `<命令>`: MCP 服务器命令
- `<参数>`: 传递给 MCP 服务器的参数
- `-- <工具参数>`: 工具名称和参数（在 `--` 之后）

### OpenAPI 参数

- `--base-url <URL>`: API 基础 URL
- `--list`: 列出所有可用端点

## 环境变量

- `MCP_API_KEY`: API 密钥认证
- `MCP_BEARER_TOKEN`: Bearer Token 认证

认证优先级：`--auth-header` > 环境变量

## 退出码

| 代码 | 含义 | 说明 |
|------|------|------|
| 0 | 成功 | 正常执行完成 |
| 1 | CLI 错误 | 参数错误或未知命令 |
| 2 | 网络错误 | 连接失败或超时 |
| 3 | 协议错误 | MCP/OpenAPI 协议错误 |
| 4 | 执行错误 | 工具调用失败 |

## 示例

### 连接远程 MCP 搜索服务

```bash
# 列出搜索服务提供的工具
mcp2cli-rs --json mcp "https://open.bigmodel.cn/api/mcp/web_search_prime/mcp" \
  --auth-header "Authorization:Bearer YOUR_TOKEN" --list

# 执行搜索
mcp2cli-rs --json mcp "https://open.bigmodel.cn/api/mcp/web_search_prime/mcp" \
  --auth-header "Authorization:Bearer YOUR_TOKEN" -- \
  web_search_prime --search_query "Rust 编程语言"
```

### 连接本地文件系统 MCP 服务器

```bash
# 列出目录内容
mcp2cli-rs --json mcp-stdio "/opt/homebrew/bin/mcp-server-filesystem" "/Users/workspace" -- \
  list_directory --path "/Users/workspace"

# 读取文件
mcp2cli-rs --json mcp-stdio "/opt/homebrew/bin/mcp-server-filesystem" "/Users/workspace" -- \
  read_file --path "/Users/workspace/README.md"
```

### 使用 OpenAPI 规范

```bash
# 从 Petstore OpenAPI 规范生成 CLI
mcp2cli-rs --json spec "https://petstore.swagger.io/v2/swagger.json" \
  --base-url "https://petstore.swagger.io/v2" --list

# 获取宠物信息
mcp2cli-rs --json spec "https://petstore.swagger.io/v2/swagger.json" \
  --base-url "https://petstore.swagger.io/v2" -- \
  getPetById --petId 1
```

## 支持的 MCP 协议

- **MCP HTTP**: Streamable HTTP 传输（JSON-RPC over HTTP POST）
- **MCP SSE**: Server-Sent Events 传输
- **MCP stdio**: 标准输入输出传输（本地进程）

## 技术栈

- **Rust**: 编程语言
- **Tokio**: 异步运行时
- **Reqwest**: HTTP 客户端
- **Clap**: CLI 参数解析
- **Serde**: JSON 序列化/反序列化

## 项目结构

```
mcp2cli-rs/
├── src/
│   ├── cli/          # CLI 参数解析
│   │   ├── args.rs
│   │   └── mod.rs
│   ├── mcp/          # MCP 协议实现
│   │   ├── client.rs    # HTTP/SSE/stdio 客户端
│   │   ├── mod.rs
│   │   └── protocol.rs  # JSON-RPC 协议类型
│   ├── openapi/      # OpenAPI 实现
│   │   ├── executor.rs  # 请求执行器
│   │   ├── mod.rs
│   │   └── spec.rs      # 规范解析
│   ├── error.rs      # 错误类型
│   ├── lib.rs        # 库入口
│   ├── main.rs       # 主程序
│   └── output.rs     # 输出格式
├── Cargo.toml
└── README.md
```

## 开发

```bash
# 编译
cargo build

# 运行测试
cargo test

# 发布构建
cargo build --release

# 格式化代码
cargo fmt

# 检查代码
cargo clippy
```

## 与上游的关系

本项目是 [mcp2cli](https://github.com/knowsuchagency/mcp2cli)（Python 版本）的 Rust 语言实现。

主要区别：
- **体积**: Rust 版本 ~2.5MB vs Python 版本 ~50MB+
- **启动速度**: Rust 版本 < 50ms
- **依赖**: Rust 版本零运行时依赖
- **协议支持**: Rust 版本专注于 MCP Streamable HTTP 和 stdio

## 跨平台兼容性

### 支持的平台

✅ **Linux** (x86_64, ARM64)
- 使用 musl 静态链接，无需 glibc
- 适用于 Alpine Linux 等轻量级容器
- 支持 Ubuntu、CentOS、Debian 等主流发行版

✅ **macOS** (Intel, Apple Silicon)
- 支持 macOS 10.15+
- 原生 Apple Silicon (M1/M2/M3) 支持

✅ **Windows** (x86_64)
- Windows 10/11 支持
- 使用 GNU 工具链构建

### 技术细节

- **TLS 库**: 使用 `rustls`（纯 Rust 实现），不依赖系统 OpenSSL
- **DNS 解析**: 使用纯 Rust 的 DNS 解析器
- **进程管理**: 使用 Tokio 跨平台进程 API
- **文件路径**: 自动处理 Windows/Unix 路径差异

### 构建目标

```bash
# 查看支持的目标平台
rustup target list

# Linux x86_64 (musl 静态链接)
cargo build --release --target x86_64-unknown-linux-musl

# Linux ARM64 (musl 静态链接)
cargo build --release --target aarch64-unknown-linux-musl

# macOS x86_64
cargo build --release --target x86_64-apple-darwin

# macOS ARM64
cargo build --release --target aarch64-apple-darwin

# Windows x86_64
cargo build --release --target x86_64-pc-windows-gnu
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

## 相关项目

- [mcp2cli](https://github.com/knowsuchagency/mcp2cli) - 原版 Python 实现
- [MCP Specification](https://modelcontextprotocol.io/) - MCP 协议规范
- [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) - 轻量级 AI Agent 框架（主要目标用户）
