use crate::error::AppError;
use crate::openapi::spec::*;
use anyhow::Result;
use reqwest::{Client, Method, header};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub struct OpenApiExecutor {
    client: Client,
    base_url: String,
    spec: OpenApiSpec,
}

impl OpenApiExecutor {
    pub fn new(spec: OpenApiSpec, base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or_else(|| spec.get_base_url());
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url,
            spec,
        })
    }

    pub fn list_operations(&self) -> Vec<(String, String, String)> {
        self.spec.get_operations()
            .into_iter()
            .map(|(path, method, op)| {
                let name = op.operation_id.clone();
                let description = op.description.clone()
                    .or(op.summary.clone())
                    .unwrap_or_else(|| format!("{} {}", method, path));
                (name, method, description)
            })
            .collect()
    }

    pub async fn execute(
        &self,
        operation_id: &str,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
        body: Option<Value>,
    ) -> Result<Value> {
        // Find operation
        let (path, method, operation) = self.find_operation(operation_id)?;
        
        // Build URL
        let url = self.build_url(&path, &path_params, &query_params)?;
        
        // Build request
        let mut request_builder = self.client.request(
            Method::from_bytes(method.as_bytes())?,
            &url
        );

        // Add auth headers if present in environment
        if let Ok(api_key) = std::env::var("MCP_API_KEY") {
            request_builder = request_builder.header("X-API-Key", api_key);
        } else if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
            request_builder = request_builder.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        // Add body for POST/PUT/PATCH
        if let Some(body_content) = body {
            request_builder = request_builder
                .header(header::CONTENT_TYPE, "application/json")
                .json(&body_content);
        }

        // Execute request
        let response = request_builder
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body_text = response.text().await
            .map_err(|e| AppError::Protocol(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            return Err(AppError::Execution(format!(
                "HTTP {}: {}", status, body_text
            )).into());
        }

        // Parse response as JSON or return text
        let result: Value = serde_json::from_str(&body_text)
            .unwrap_or_else(|_| Value::String(body_text));

        Ok(result)
    }

    fn find_operation(&self, operation_id: &str) -> Result<(String, String, Operation)> {
        for (path, item) in &self.spec.paths {
            if let Some(op) = &item.get {
                if op.operation_id == operation_id {
                    return Ok((path.clone(), "GET".to_string(), op.clone()));
                }
            }
            if let Some(op) = &item.post {
                if op.operation_id == operation_id {
                    return Ok((path.clone(), "POST".to_string(), op.clone()));
                }
            }
            if let Some(op) = &item.put {
                if op.operation_id == operation_id {
                    return Ok((path.clone(), "PUT".to_string(), op.clone()));
                }
            }
            if let Some(op) = &item.delete {
                if op.operation_id == operation_id {
                    return Ok((path.clone(), "DELETE".to_string(), op.clone()));
                }
            }
            if let Some(op) = &item.patch {
                if op.operation_id == operation_id {
                    return Ok((path.clone(), "PATCH".to_string(), op.clone()));
                }
            }
        }
        
        Err(AppError::Cli(format!("Operation '{}' not found", operation_id)).into())
    }

    fn build_url(
        &self,
        path: &str,
        path_params: &HashMap<String, String>,
        query_params: &HashMap<String, String>,
    ) -> Result<String> {
        // Replace path parameters
        let mut final_path = path.to_string();
        for (key, value) in path_params {
            final_path = final_path.replace(&format!("{{{}}}", key), value);
        }

        let mut url = format!("{}{}", self.base_url.trim_end_matches('/'), final_path);

        // Add query parameters
        if !query_params.is_empty() {
            let query_string: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&query_string.join("&"));
        }

        Ok(url)
    }

    pub fn get_operation_schema(&self, operation_id: &str) -> Result<Value> {
        let (_, _, operation) = self.find_operation(operation_id)?;
        
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Add parameters as properties
        if let Some(params) = &operation.parameters {
            for param in params {
                let param_name = &param.name;
                let param_schema = param.schema.clone().unwrap_or_else(|| {
                    serde_json::json!({"type": "string"})
                });
                
                properties.insert(param_name.clone(), param_schema);
                
                if param.required.unwrap_or(false) {
                    required.push(param_name.clone());
                }
            }
        }

        // Add request body if present
        if let Some(request_body) = &operation.request_body {
            if let Some(content) = request_body.content.get("application/json") {
                if let Some(schema) = &content.schema {
                    properties.insert("body".to_string(), schema.clone());
                    if request_body.required.unwrap_or(false) {
                        required.push("body".to_string());
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        }))
    }
}
