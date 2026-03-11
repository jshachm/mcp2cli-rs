use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mcp2cli-rs")]
#[command(about = "Minimal, stateless CLI for MCP and OpenAPI")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Request timeout in seconds
    #[arg(long, global = true, default_value = "30")]
    pub timeout: u64,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    /// MCP HTTP/SSE mode
    Mcp(McpArgs),

    /// MCP stdio mode
    #[command(name = "mcp-stdio")]
    McpStdio(McpStdioArgs),

    /// OpenAPI mode
    #[command(name = "spec")]
    OpenApi(OpenApiArgs),
}

#[derive(Args, Debug)]
pub struct McpArgs {
    /// MCP server URL (HTTP or SSE endpoint)
    pub url: String,

    /// List available tools
    #[arg(long)]
    pub list: bool,

    /// HTTP header as Name:Value
    #[arg(long = "auth-header")]
    pub auth_headers: Vec<String>,

    /// Tool name to call
    #[arg(trailing_var_arg = true)]
    pub tool_args: Vec<String>,
}

#[derive(Args, Debug)]
pub struct McpStdioArgs {
    /// Command to spawn MCP server
    pub command: String,

    /// List available tools
    #[arg(long)]
    pub list: bool,

    /// Arguments for the command (before --)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,

    /// Tool name and arguments (after --)
    #[arg(last = true)]
    pub tool_args: Vec<String>,
}

#[derive(Args, Debug)]
pub struct OpenApiArgs {
    /// OpenAPI spec URL or file path
    pub spec: String,

    /// Base URL for API calls
    #[arg(long)]
    pub base_url: Option<String>,

    /// List available endpoints
    #[arg(long)]
    pub list: bool,

    /// Operation ID to call
    #[arg(trailing_var_arg = true)]
    pub operation_args: Vec<String>,
}

impl Cli {
    pub fn parse_tool_args(args: &[String]) -> (Option<String>, Vec<(String, String)>) {
        let mut tool_name = None;
        let mut params = Vec::new();
        let mut i = 0;

        // Skip leading "--" if present (clap separator)
        while i < args.len() && args[i] == "--" {
            i += 1;
        }

        while i < args.len() {
            let arg = &args[i];

            if !arg.starts_with("--") && tool_name.is_none() {
                // First non-flag argument is the tool name
                tool_name = Some(arg.clone());
            } else if arg.starts_with("--") && i + 1 < args.len() {
                // --key value pair
                let key = arg.trim_start_matches("--").to_string();
                let value = args[i + 1].clone();
                params.push((key, value));
                i += 1;
            }

            i += 1;
        }

        (tool_name, params)
    }
}
