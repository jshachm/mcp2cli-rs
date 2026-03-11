#!/bin/bash

# Example usage of mcp2cli-rs

echo "=== MCP HTTP Example ==="
./mcp2cli-rs mcp https://api.example.com/mcp --list --json

echo "=== MCP stdio Example ==="
./mcp2cli-rs mcp-stdio "npx @modelcontextprotocol/server-filesystem /tmp" --list --json

echo "=== OpenAPI Example ==="
./mcp2cli-rs spec ./examples/petstore.json --base-url https://api.example.com --list --json
