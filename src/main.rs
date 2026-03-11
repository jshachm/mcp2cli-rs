mod cli;
mod error;
mod mcp;
mod openapi;
mod output;

use clap::Parser;
use cli::args::{Cli, Mode};
use error::{AppError, ExitCode};
use mcp::client::{McpHttpClient, McpSseClient, McpStdioClient};
use openapi::spec::OpenApiSpec;
use openapi::executor::OpenApiExecutor;
use output::{Content, Protocol, Tool, ToolManifest, ToolResponse};
use serde_json::Value;
use std::collections::HashMap;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    let result = match cli.mode {
        Mode::Mcp(args) => handle_mcp(args, cli.json, cli.timeout).await,
        Mode::McpStdio(args) => handle_mcp_stdio(args, cli.json).await,
        Mode::OpenApi(args) => handle_openapi(args, cli.json).await,
    };

    match result {
        Ok(()) => process::exit(ExitCode::Success.as_i32()),
        Err(e) => {
            let exit_code = e.exit_code();
            
            if cli.json {
                let error_response = ToolResponse::error(
                    "unknown".to_string(),
                    &format!("{}", e),
                    &e.to_string()
                );
                error_response.print_json();
            } else {
                eprintln!("Error: {}", e);
            }
            
            process::exit(exit_code.as_i32());
        }
    }
}

async fn handle_mcp(args: cli::args::McpArgs, _json: bool, timeout: u64) -> Result<(), AppError> {
    let url = &args.url;

    // Parse auth headers from args
    let mut headers = std::collections::HashMap::new();
    for header_str in &args.auth_headers {
        if let Some((key, value)) = header_str.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    // Determine if it's SSE or HTTP based on URL or content
    let is_sse = url.ends_with("/sse") || url.contains("/sse?");

    if args.list {
        // List tools
        let tools = if is_sse {
            let mut client = McpSseClient::new(url, timeout)
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.connect().await
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.list_tools().await
                .map_err(|e| AppError::Protocol(e.to_string()))?
        } else {
            let client = McpHttpClient::new_with_headers(url, timeout, headers)
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.list_tools().await
                .map_err(|e| AppError::Protocol(e.to_string()))?
        };

        let manifest = ToolManifest {
            protocol: Protocol::Mcp,
            source: url.clone(),
            version: Some("2024-11-05".to_string()),
            tools: tools.into_iter().map(|t| Tool {
                name: t.name.clone(),
                original_name: Some(t.name),
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema,
                output_type: None,
            }).collect(),
        };

        manifest.print_json();
        Ok(())
    } else {
        // Execute tool
        let (tool_name, params) = Cli::parse_tool_args(&args.tool_args);
        let tool_name = tool_name.ok_or_else(|| AppError::Cli("Tool name required".to_string()))?;
        
        // Build arguments JSON
        let mut arguments = serde_json::Map::new();
        for (key, value) in params {
            // Try to parse as JSON, otherwise treat as string
            let value: Value = serde_json::from_str(&value).unwrap_or_else(|_| Value::String(value));
            arguments.insert(key, value);
        }

        let result = if is_sse {
            let mut client = McpSseClient::new(url, timeout)
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.connect().await
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.call_tool(&tool_name, Value::Object(arguments)).await
                .map_err(|e| AppError::Execution(e.to_string()))?
        } else {
            let client = McpHttpClient::new_with_headers(url, timeout, headers)
                .map_err(|e| AppError::Network(e.to_string()))?;
            client.call_tool(&tool_name, Value::Object(arguments)).await
                .map_err(|e| AppError::Execution(e.to_string()))?
        };

        let content: Vec<Content> = result.content.into_iter().map(|c| match c {
            mcp::protocol::McpContent::Text { text } => Content::Text { text },
            mcp::protocol::McpContent::Image { data, mime_type } => Content::Image { data, mime_type },
            mcp::protocol::McpContent::Resource { resource } => Content::Text { 
                text: resource.text.unwrap_or_else(|| format!("Resource: {}", resource.uri)) 
            },
        }).collect();

        let response = ToolResponse::success(tool_name, content);
        response.print_json();
        Ok(())
    }
}

async fn handle_mcp_stdio(args: cli::args::McpStdioArgs, json: bool) -> Result<(), AppError> {
    let mut client = McpStdioClient::spawn(&args.command, &args.args).await
        .map_err(|e| AppError::Execution(e.to_string()))?;

    if args.list {
        let tools = client.list_tools().await
            .map_err(|e| AppError::Protocol(e.to_string()))?;

        let manifest = ToolManifest {
            protocol: Protocol::Mcp,
            source: format!("stdio: {}", args.command),
            version: Some("2024-11-05".to_string()),
            tools: tools.into_iter().map(|t| Tool {
                name: t.name.clone(),
                original_name: Some(t.name),
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema,
                output_type: None,
            }).collect(),
        };

        manifest.print_json();
        let _ = client.close().await;
        Ok(())
    } else {
        // Execute tool
        let (tool_name, params) = Cli::parse_tool_args(&args.tool_args);
        let tool_name = tool_name.ok_or_else(|| AppError::Cli("Tool name required".to_string()))?;
        
        let mut arguments = serde_json::Map::new();
        for (key, value) in params {
            let value: Value = serde_json::from_str(&value).unwrap_or_else(|_| Value::String(value));
            arguments.insert(key, value);
        }

        let result = client.call_tool(&tool_name, Value::Object(arguments)).await
            .map_err(|e| AppError::Execution(e.to_string()))?;

        let content: Vec<Content> = result.content.into_iter().map(|c| match c {
            mcp::protocol::McpContent::Text { text } => Content::Text { text },
            mcp::protocol::McpContent::Image { data, mime_type } => Content::Image { data, mime_type },
            mcp::protocol::McpContent::Resource { resource } => Content::Text { 
                text: resource.text.unwrap_or_else(|| format!("Resource: {}", resource.uri)) 
            },
        }).collect();

        let response = ToolResponse::success(tool_name, content);
        response.print_json();
        let _ = client.close().await;
        Ok(())
    }
}

async fn handle_openapi(args: cli::args::OpenApiArgs, json: bool) -> Result<(), AppError> {
    let spec = OpenApiSpec::load(&args.spec).await
        .map_err(|e| AppError::Network(e.to_string()))?;
    
    let executor = OpenApiExecutor::new(spec, args.base_url)
        .map_err(|e| AppError::Protocol(e.to_string()))?;

    if args.list {
        let operations = executor.list_operations();
        
        let tools: Vec<Tool> = operations.into_iter().map(|(name, method, description)| {
            let schema = executor.get_operation_schema(&name)
                .unwrap_or_else(|_| serde_json::json!({"type": "object"}));
            
            Tool {
                name: name.clone(),
                original_name: Some(name),
                description: format!("{} {}", method, description),
                input_schema: schema,
                output_type: None,
            }
        }).collect();

        let manifest = ToolManifest {
            protocol: Protocol::OpenApi,
            source: args.spec.clone(),
            version: Some("3.0".to_string()),
            tools,
        };

        manifest.print_json();
        Ok(())
    } else {
        // Execute operation
        let (operation_id, params) = Cli::parse_tool_args(&args.operation_args);
        let operation_id = operation_id.ok_or_else(|| AppError::Cli("Operation ID required".to_string()))?;
        
        let mut path_params = HashMap::new();
        let mut query_params = HashMap::new();
        let mut body = None;

        for (key, value) in params {
            if key == "body" {
                body = serde_json::from_str(&value).ok();
            } else if value.contains('/') || value.contains(':') {
                // Heuristic: likely a path param
                path_params.insert(key, value);
            } else {
                query_params.insert(key, value);
            }
        }

        let result = executor.execute(&operation_id, path_params, query_params, body).await
            .map_err(|e| AppError::Execution(e.to_string()))?;

        let content = vec![Content::Json { data: result }];
        let response = ToolResponse::success(operation_id, content);
        response.print_json();
        Ok(())
    }
}