use crate::error::AppError;
use crate::mcp::protocol::*;
use anyhow::Result;
use reqwest::{Client, header, Response};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command, ChildStdin, ChildStdout};
use url::Url;

/// Parse HTTP response as JSON or SSE format
async fn parse_response(response: Response) -> Result<JsonRpcResponse> {
    let content_type = response.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    // Check if response is SSE
    if content_type.contains("text/event-stream") {
        // Parse SSE response
        let body = response.text().await
            .map_err(|e| AppError::Protocol(format!("Failed to read SSE response: {}", e)))?;
        
        // Extract data from SSE format
        // Format: id:1\nevent:message\ndata:{"jsonrpc":...}
        let data_line = body.lines()
            .find(|line| line.starts_with("data:"))
            .ok_or_else(|| AppError::Protocol("No data line in SSE response".to_string()))?;
        
        let json_data = data_line.trim_start_matches("data:").trim();
        let jsonrpc_resp: JsonRpcResponse = serde_json::from_str(json_data)
            .map_err(|e| AppError::Protocol(format!("Failed to parse SSE JSON data: {}", e)))?;
        
        return Ok(jsonrpc_resp);
    }
    
    // Parse as regular JSON
    let jsonrpc_resp: JsonRpcResponse = response
        .json()
        .await
        .map_err(|e| AppError::Protocol(format!("Failed to parse JSON response: {}", e)))?;
    
    Ok(jsonrpc_resp)
}

pub struct McpHttpClient {
    client: Client,
    base_url: Url,
    timeout: Duration,
    headers: std::collections::HashMap<String, String>,
}

impl McpHttpClient {
    pub fn new(base_url: impl AsRef<str>, timeout_secs: u64) -> Result<Self> {
        Self::new_with_headers(base_url, timeout_secs, HashMap::new())
    }
    
    pub fn new_with_headers(
        base_url: impl AsRef<str>,
        timeout_secs: u64,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let base_url = Url::parse(base_url.as_ref())?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            base_url,
            timeout: Duration::from_secs(timeout_secs),
            headers,
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        // MCP Streamable HTTP: POST to base URL with JSON-RPC message
        let url = self.base_url.to_string();
        
        let mut request = self.client.post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        
        // Add auth if present in environment
        if let Ok(api_key) = std::env::var("MCP_API_KEY") {
            request = request.header("X-API-Key", api_key);
        } else if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let jsonrpc_req = JsonRpcRequest::new(1, "tools/list", json!({}));
        
        let response = request
            .json(&jsonrpc_req)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AppError::Protocol(
                format!("HTTP {}: {}", response.status(), response.text().await.unwrap_or_default())
            ).into());
        }

        // Parse response based on content type
        let jsonrpc_resp = parse_response(response).await?;

        if let Some(error) = jsonrpc_resp.error {
            return Err(AppError::Protocol(format!("JSON-RPC error {}: {}", error.code, error.message)).into());
        }

        let result: ListToolsResult = serde_json::from_value(
            jsonrpc_resp.result.ok_or_else(|| AppError::Protocol("Empty result".to_string()))?
        )?;

        Ok(result.tools)
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<CallToolResult> {
        // MCP Streamable HTTP: POST to base URL with JSON-RPC message
        let url = self.base_url.to_string();
        
        let mut request = self.client.post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        
        // Add auth if present in environment
        if let Ok(api_key) = std::env::var("MCP_API_KEY") {
            request = request.header("X-API-Key", api_key);
        } else if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });

        let jsonrpc_req = JsonRpcRequest::new(1, "tools/call", params);
        
        let response = request
            .json(&jsonrpc_req)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AppError::Execution(
                format!("HTTP {}: {}", response.status(), response.text().await.unwrap_or_default())
            ).into());
        }

        // Parse response based on content type
        let jsonrpc_resp = parse_response(response).await?;

        if let Some(error) = jsonrpc_resp.error {
            return Err(AppError::Execution(format!("JSON-RPC error {}: {}", error.code, error.message)).into());
        }

        let result: CallToolResult = serde_json::from_value(
            jsonrpc_resp.result.ok_or_else(|| AppError::Protocol("Empty result".to_string()))?
        )?;

        Ok(result)
    }
}

pub struct McpSseClient {
    client: Client,
    sse_endpoint: Url,
    message_endpoint: Option<Url>,
    timeout: Duration,
}

impl McpSseClient {
    pub fn new(sse_url: impl AsRef<str>, timeout_secs: u64) -> Result<Self> {
        let sse_endpoint = Url::parse(sse_url.as_ref())?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            sse_endpoint,
            message_endpoint: None,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        // SSE connection to discover message endpoint
        let response = self.client
            .get(self.sse_endpoint.as_str())
            .send()
            .await
            .map_err(|e| AppError::Network(format!("SSE connection failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Network(format!("SSE HTTP {}", response.status())).into());
        }

        // Parse SSE to get message endpoint (simplified - in real impl, parse the stream)
        // For now, assume message endpoint is derived from SSE endpoint
        let mut message_url = self.sse_endpoint.clone();
        message_url.set_path("/message");
        self.message_endpoint = Some(message_url);

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let message_url = self.message_endpoint
            .as_ref()
            .ok_or_else(|| AppError::Protocol("Not connected".to_string()))?;

        let jsonrpc_req = JsonRpcRequest::new(1, "tools/list", json!({}));
        
        let mut request = self.client.post(message_url.as_str())
            .header(header::CONTENT_TYPE, "application/json");

        if let Ok(api_key) = std::env::var("MCP_API_KEY") {
            request = request.header("X-API-Key", api_key);
        } else if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let response = request
            .json(&jsonrpc_req)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        let jsonrpc_resp: JsonRpcResponse = response.json().await
            .map_err(|e| AppError::Protocol(format!("Parse error: {}", e)))?;

        if let Some(error) = jsonrpc_resp.error {
            return Err(AppError::Protocol(format!("JSON-RPC {}: {}", error.code, error.message)).into());
        }

        let result: ListToolsResult = serde_json::from_value(
            jsonrpc_resp.result.ok_or_else(|| AppError::Protocol("Empty result".to_string()))?
        )?;

        Ok(result.tools)
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<CallToolResult> {
        let message_url = self.message_endpoint
            .as_ref()
            .ok_or_else(|| AppError::Protocol("Not connected".to_string()))?;

        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });

        let jsonrpc_req = JsonRpcRequest::new(1, "tools/call", params);
        
        let mut request = self.client.post(message_url.as_str())
            .header(header::CONTENT_TYPE, "application/json");

        if let Ok(api_key) = std::env::var("MCP_API_KEY") {
            request = request.header("X-API-Key", api_key);
        } else if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let response = request
            .json(&jsonrpc_req)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        let jsonrpc_resp: JsonRpcResponse = response.json().await
            .map_err(|e| AppError::Protocol(format!("Parse error: {}", e)))?;

        if let Some(error) = jsonrpc_resp.error {
            return Err(AppError::Execution(format!("JSON-RPC {}: {}", error.code, error.message)).into());
        }

        let result: CallToolResult = serde_json::from_value(
            jsonrpc_resp.result.ok_or_else(|| AppError::Protocol("Empty result".to_string()))?
        )?;

        Ok(result)
    }
}

pub struct McpStdioClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: u64,
}

impl McpStdioClient {
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut process = cmd.spawn()
            .map_err(|e| AppError::Execution(format!("Failed to spawn process: {}", e)))?;

        let stdin = process.stdin.take()
            .ok_or_else(|| AppError::Protocol("Failed to open stdin".to_string()))?;
        
        let stdout = process.stdout.take()
            .ok_or_else(|| AppError::Protocol("Failed to open stdout".to_string()))?;

        Ok(Self {
            process,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: 0,
        })
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.request_id += 1;
        let request = JsonRpcRequest::new(self.request_id, method, params);
        let request_json = serde_json::to_string(&request)?;
        
        // Write request with newline
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response line
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await
            .map_err(|e| AppError::Protocol(format!("Failed to read response: {}", e)))?;

        if response_line.is_empty() {
            return Err(AppError::Protocol("Empty response from server".to_string()).into());
        }

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| AppError::Protocol(format!("Failed to parse JSON-RPC response: {}", e)))?;

        if response.id != self.request_id {
            return Err(AppError::Protocol(format!("Request ID mismatch: expected {}, got {}", 
                self.request_id, response.id)).into());
        }

        if let Some(error) = response.error {
            return Err(AppError::Execution(format!("JSON-RPC error {}: {}", error.code, error.message)).into());
        }

        response.result.ok_or_else(|| AppError::Protocol("Empty result".to_string()).into())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.send_request("tools/list", json!({})).await?;
        let list_result: ListToolsResult = serde_json::from_value(result)?;
        Ok(list_result.tools)
    }

    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<CallToolResult> {
        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });
        let result = self.send_request("tools/call", params).await?;
        let call_result: CallToolResult = serde_json::from_value(result)?;
        Ok(call_result)
    }

    pub async fn close(mut self) -> Result<()> {
        let _ = self.stdin.shutdown().await;
        let _ = self.process.wait().await;
        Ok(())
    }
}
