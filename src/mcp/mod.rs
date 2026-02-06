pub mod types;

use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::db::Database;
use types::{JsonRpcRequest, JsonRpcResponse, JSONRPC_VERSION};

pub struct McpServer {
    db: Database,
}

impl McpServer {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn run(&self) -> Result<()> {
        eprintln!("MCP server starting on stdio");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut lines = BufReader::new(stdin).lines();

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                continue;
            }

            if let Some(response) = self.process_message(&line) {
                let mut out = serde_json::to_string(&response)?;
                out.push('\n');
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
            }
        }

        eprintln!("MCP server shutting down (stdin closed)");
        Ok(())
    }

    fn process_message(&self, line: &str) -> Option<JsonRpcResponse> {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Parse error: {e}");
                return Some(JsonRpcResponse::parse_error(format!("Parse error: {e}")));
            }
        };

        if request.jsonrpc != JSONRPC_VERSION {
            let id = request.id.unwrap_or(Value::Null);
            return Some(JsonRpcResponse::invalid_request(
                id,
                format!("Invalid jsonrpc version: {}", request.jsonrpc),
            ));
        }

        match request.id {
            None => {
                // Notification — no response
                self.handle_notification(&request);
                None
            }
            Some(Value::Null) => {
                // MCP spec: id MUST NOT be null
                Some(JsonRpcResponse::invalid_request(
                    Value::Null,
                    "Request id must not be null",
                ))
            }
            Some(ref id) if id.is_string() || id.is_number() => {
                self.handle_request(&request, id.clone())
            }
            Some(id) => Some(JsonRpcResponse::invalid_request(
                id,
                "Request id must be a string or number",
            )),
        }
    }

    fn handle_notification(&self, request: &JsonRpcRequest) {
        eprintln!("Received notification: {}", request.method);
    }

    fn handle_request(&self, request: &JsonRpcRequest, id: Value) -> Option<JsonRpcResponse> {
        let _ = &self.db; // suppress unused warning until handlers are added
        Some(JsonRpcResponse::method_not_found(id, &request.method))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_server() -> (McpServer, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (McpServer::new(db), dir)
    }

    #[test]
    fn test_valid_request_returns_method_not_found() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let resp = server.process_message(line).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, types::METHOD_NOT_FOUND);
        assert!(err.message.contains("test"));
    }

    #[test]
    fn test_malformed_json_returns_parse_error() {
        let (server, _dir) = test_server();
        let resp = server.process_message("not json").unwrap();
        assert_eq!(resp.error.unwrap().code, types::PARSE_ERROR);
        assert!(resp.id.is_null());
    }

    #[test]
    fn test_invalid_jsonrpc_version() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"1.0","method":"test","id":1}"#;
        let resp = server.process_message(line).unwrap();
        assert_eq!(resp.error.unwrap().code, types::INVALID_REQUEST);
    }

    #[test]
    fn test_notification_returns_none() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(server.process_message(line).is_none());
    }

    #[test]
    fn test_request_with_null_id_returns_invalid_request() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"test","id":null}"#;
        let resp = server.process_message(line).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, types::INVALID_REQUEST);
        assert!(err.message.contains("null"));
    }

    #[test]
    fn test_request_with_invalid_id_type_returns_invalid_request() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"test","id":true}"#;
        let resp = server.process_message(line).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, types::INVALID_REQUEST);
        assert!(err.message.contains("string or number"));
    }
}
