# mcp2cli-rs

[English](README.md) | [中文](README.zh-CN.md)

Minimal, stateless CLI tool for MCP (Model Context Protocol) and OpenAPI.

A Rust implementation of [mcp2cli](https://github.com/knowsuchagency/mcp2cli).

## Features

- **Lightweight**: Single binary < 3MB
- **Stateless**: No cache, no session, no persistence
- **Machine-first**: JSON output for program parsing
- **Zero dependencies**: Static linking, no runtime libraries

## Installation

```bash
cargo build --release
# Binary: target/release/mcp2cli-rs
```

## Usage

### MCP HTTP Mode

```bash
# List tools
mcp2cli-rs mcp <URL> --list --json

# Call tool
mcp2cli-rs mcp <URL> <tool-name> --arg value --json
```

### MCP stdio Mode

```bash
# List tools
mcp2cli-rs mcp-stdio "npx @modelcontextprotocol/server-filesystem /tmp" --list --json

# Call tool
mcp2cli-rs mcp-stdio "npx @modelcontextprotocol/server-filesystem /tmp" read_file --path /tmp/test.txt --json
```

### OpenAPI Mode

```bash
# List operations
mcp2cli-rs spec <URL|FILE> --base-url <URL> --list --json

# Call operation
mcp2cli-rs spec <URL|FILE> --base-url <URL> <operation-id> --param value --json
```

## Environment Variables

- `MCP_API_KEY`: API key for authentication
- `MCP_BEARER_TOKEN`: Bearer token for authentication

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | CLI Error |
| 2 | Network Error |
| 3 | Protocol Error |
| 4 | Execution Error |

## License

MIT
