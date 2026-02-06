pub mod tools;
pub mod types;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::db::Database;
use types::{JsonRpcRequest, JsonRpcResponse, INVALID_PARAMS, JSONRPC_VERSION};

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
        match request.method.as_str() {
            "notifications/initialized" => eprintln!("Client initialized"),
            _ => eprintln!("Received notification: {}", request.method),
        }
    }

    fn handle_request(&self, request: &JsonRpcRequest, id: Value) -> Option<JsonRpcResponse> {
        match request.method.as_str() {
            "initialize" => Some(self.handle_initialize(id)),
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            "tools/list" => Some(self.handle_tools_list(id)),
            "tools/call" => Some(self.handle_tools_call(request, id)),
            _ => Some(JsonRpcResponse::method_not_found(id, &request.method)),
        }
    }

    fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "blueprint",
                    "version": "0.1.0"
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, json!({ "tools": tools::tool_definitions() }))
    }

    fn handle_tools_call(&self, request: &JsonRpcRequest, id: Value) -> JsonRpcResponse {
        let Some(params) = &request.params else {
            return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing params");
        };

        let Some(name) = params.get("name").and_then(|n| n.as_str()) else {
            return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing tool name");
        };

        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match tools::dispatch_tool(name, &args, &self.db) {
            Some(result) => JsonRpcResponse::success(id, result),
            None => JsonRpcResponse::error(
                id,
                INVALID_PARAMS,
                format!("Unknown tool: {name}"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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

    #[test]
    fn test_initialize_returns_server_info() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
        let resp = server.process_message(line).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2025-06-18");
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(result["serverInfo"]["name"], "blueprint");
        assert_eq!(result["serverInfo"]["version"], "0.1.0");
    }

    #[test]
    fn test_ping_returns_empty_object() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"ping","id":1}"#;
        let resp = server.process_message(line).unwrap();
        assert_eq!(resp.result.unwrap(), json!({}));
    }

    #[test]
    fn test_tools_list_returns_19_tools() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let resp = server.process_message(line).unwrap();
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 19);
    }

    #[test]
    fn test_tools_call_known_tool() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"create_project","arguments":{"name":"test","description":"desc"}}}"#;
        let resp = server.process_message(line).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("create_project"));
    }

    #[test]
    fn test_tools_call_unknown_tool() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"nonexistent"}}"#;
        let resp = server.process_message(line).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("nonexistent"));
    }

    #[test]
    fn test_tools_call_missing_params() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"tools/call","id":1}"#;
        let resp = server.process_message(line).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
    }

    #[test]
    fn test_initialized_notification() {
        let (server, _dir) = test_server();
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(server.process_message(line).is_none());
    }
}
