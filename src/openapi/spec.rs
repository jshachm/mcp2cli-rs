use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,
    pub paths: BTreeMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Info {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: BTreeMap<String, Response>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String, // query, header, path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: BTreeMap<String, MediaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<BTreeMap<String, MediaType>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<BTreeMap<String, Value>>,
}

impl OpenApiSpec {
    pub async fn load(source: &str) -> Result<Self> {
        let content = if source.starts_with("http://") || source.starts_with("https://") {
            // Load from URL
            let client = reqwest::Client::new();
            let response = client.get(source).send().await?;
            if !response.status().is_success() {
                return Err(anyhow::anyhow!("Failed to fetch OpenAPI spec: HTTP {}", response.status()));
            }
            response.text().await?
        } else {
            // Load from file
            tokio::fs::read_to_string(source).await?
        };

        let spec: OpenApiSpec = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenAPI spec: {}", e))?;
        
        Ok(spec)
    }

    pub fn get_base_url(&self) -> String {
        self.servers
            .as_ref()
            .and_then(|s| s.first())
            .map(|s| s.url.clone())
            .unwrap_or_else(|| "http://localhost".to_string())
    }

    pub fn get_operations(&self) -> Vec<(String, String, Operation)> {
        let mut operations = Vec::new();
        
        for (path, item) in &self.paths {
            if let Some(op) = &item.get {
                operations.push((path.clone(), "GET".to_string(), op.clone()));
            }
            if let Some(op) = &item.post {
                operations.push((path.clone(), "POST".to_string(), op.clone()));
            }
            if let Some(op) = &item.put {
                operations.push((path.clone(), "PUT".to_string(), op.clone()));
            }
            if let Some(op) = &item.delete {
                operations.push((path.clone(), "DELETE".to_string(), op.clone()));
            }
            if let Some(op) = &item.patch {
                operations.push((path.clone(), "PATCH".to_string(), op.clone()));
            }
        }
        
        operations
    }
}