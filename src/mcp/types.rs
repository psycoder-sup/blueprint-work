use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";

// JSON-RPC 2.0 standard error codes
pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
#[allow(dead_code)]
pub const INTERNAL_ERROR: i64 = -32603;

/// Deserializes a JSON value into `Some(value)`, preserving explicit `null` as
/// `Some(Value::Null)`. Absent fields default to `None` via `#[serde(default)]`.
fn deserialize_optional_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_value")]
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Value, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::error(Value::Null, PARSE_ERROR, message)
    }

    pub fn invalid_request(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, INVALID_REQUEST, message)
    }

    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(id, METHOD_NOT_FOUND, format!("Method not found: {method}"))
    }

    #[allow(dead_code)]
    pub fn internal_error(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, INTERNAL_ERROR, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_request_with_number_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test");
        assert_eq!(req.id, Some(json!(1)));
        assert!(req.params.is_none());
    }

    #[test]
    fn test_deserialize_request_with_string_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"test","id":"abc"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.id, Some(json!("abc")));
    }

    #[test]
    fn test_deserialize_notification() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert!(req.id.is_none());
    }

    #[test]
    fn test_deserialize_request_with_params() {
        let raw = r#"{"jsonrpc":"2.0","method":"test","id":1,"params":{"key":"value"}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.params, Some(json!({"key": "value"})));
    }

    #[test]
    fn test_serialize_success_response() {
        let resp = JsonRpcResponse::success(json!(1), json!({"status": "ok"}));
        let serialized = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();
        assert!(parsed.get("result").is_some());
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_serialize_error_response() {
        let resp = JsonRpcResponse::error(json!(1), METHOD_NOT_FOUND, "Method not found: test");
        let serialized = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();
        assert!(parsed.get("error").is_some());
        assert!(parsed.get("result").is_none());
    }

    #[test]
    fn test_response_is_single_line() {
        let resp = JsonRpcResponse::success(json!(1), json!({"nested": {"deep": true}}));
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains('\n'), "response must not contain newlines");
    }

    #[test]
    fn test_parse_error_has_null_id() {
        let resp = JsonRpcResponse::parse_error("bad json");
        let serialized = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();
        assert!(parsed["id"].is_null());
    }

    #[test]
    fn test_method_not_found_includes_method_name() {
        let resp = JsonRpcResponse::method_not_found(json!(1), "tools/call");
        assert!(resp.error.unwrap().message.contains("tools/call"));
    }
}
