use anyhow::{bail, ensure, Context, Result};
use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Client, Response, StatusCode, Url,
};
use serde_json::{json, Map, Value};

use super::{super::read_response, digest, Estimate};

const PROTOCOL_VERSION: &str = "2025-11-25";
const ACCEPT_TYPES: &str = "application/json, text/event-stream";

pub(super) async fn estimate(
    client: &Client,
    url: &Url,
    authorization: HeaderValue,
    model: &str,
    inputs: &Map<String, Value>,
) -> Result<(Estimate, String)> {
    let post = || {
        client
            .post(url.clone())
            .header(ACCEPT, ACCEPT_TYPES)
            .header(AUTHORIZATION, authorization.clone())
    };
    let init = post()
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "tokencraft-pricing", "version": env!("CARGO_PKG_VERSION")}
            }
        }))
        .send()
        .await
        .context("initialize Kinovi estimate client")?;
    let session = init
        .headers()
        .get("mcp-session-id")
        .cloned()
        .map(|mut header| {
            header.set_sensitive(true);
            header
        });
    let (init, _) = response_result(init, 1).await?;
    let version = init["protocolVersion"]
        .as_str()
        .filter(|version| matches!(*version, PROTOCOL_VERSION | "2025-06-18" | "2025-03-26"))
        .context("unsupported Kinovi MCP protocol version")?;
    ensure!(
        init["capabilities"]["tools"].is_object(),
        "Kinovi MCP does not support tools"
    );
    let session_post = || {
        let request = post().header("mcp-protocol-version", version);
        match &session {
            Some(session) => request.header("mcp-session-id", session.clone()),
            None => request,
        }
    };
    let accepted = session_post()
        .json(&json!({
            "jsonrpc": "2.0", "method": "notifications/initialized"
        }))
        .send()
        .await
        .context("acknowledge Kinovi MCP initialization")?;
    ensure!(
        accepted.status() == StatusCode::ACCEPTED,
        "Kinovi MCP initialization was not accepted"
    );

    let response = session_post()
        .json(&json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "estimate_cost", "arguments": {"modelId": model, "inputs": inputs}
            }
        }))
        .send()
        .await
        .context("read Kinovi cost estimate")?;
    let (mut result, sha256) = response_result(response, 2).await?;
    ensure!(
        result["isError"].is_null() || result["isError"] == false,
        "Kinovi rejected the cost estimate"
    );
    let estimate = if result["structuredContent"].is_object() {
        serde_json::from_value(result["structuredContent"].take())
    } else {
        let content = result["content"]
            .as_array()
            .filter(|items| items.len() == 1 && items[0]["type"] == "text")
            .context("Kinovi estimate has no structured or JSON text result")?;
        let text = content[0]["text"]
            .as_str()
            .context("Kinovi estimate is missing text")?;
        serde_json::from_str(text)
    }
    .map_err(|_| anyhow::anyhow!("invalid Kinovi estimate result"))?;
    Ok((estimate, sha256))
}

async fn response_result(response: Response, id: u64) -> Result<(Value, String)> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.split(';').next())
        .map(str::trim);
    let is_stream = content_type == Some("text/event-stream");
    ensure!(
        is_stream || content_type == Some("application/json"),
        "unexpected Kinovi MCP response type"
    );
    let bytes = read_response(response).await?;
    let message = if is_stream {
        sse_message(std::str::from_utf8(&bytes)?, id)?
    } else {
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid Kinovi MCP JSON"))?
    };
    ensure!(
        message["jsonrpc"] == "2.0" && message["id"].as_u64() == Some(id),
        "unexpected Kinovi MCP response ID"
    );
    ensure!(message.get("error").is_none(), "Kinovi MCP request failed");
    let mut message = message;
    ensure!(message["result"].is_object(), "missing Kinovi MCP result");
    Ok((message["result"].take(), digest(&bytes)))
}

fn sse_message(body: &str, id: u64) -> Result<Value> {
    let mut fields = Vec::new();
    for line in body.split_terminator('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() && !fields.is_empty() {
            let value: Value = serde_json::from_str(&fields.join("\n"))
                .map_err(|_| anyhow::anyhow!("invalid Kinovi MCP event"))?;
            if value["id"].as_u64() == Some(id) && value.get("method").is_none() {
                return Ok(value);
            }
            fields.clear();
        } else if let Some(field) = line.strip_prefix("data:") {
            fields.push(field.strip_prefix(' ').unwrap_or(field));
        }
    }
    bail!("Kinovi MCP stream ended without the estimate response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::HeaderMap, response::IntoResponse, routing::post, Json, Router};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn authenticates_and_only_calls_the_estimate_tool() {
        let server = Router::new().route("/mcp", post(|headers: HeaderMap, Json(message): Json<Value>| async move {
            assert_eq!(headers[AUTHORIZATION], "Bearer fixture-only");
            assert_eq!(headers[ACCEPT], ACCEPT_TYPES);
            assert_eq!(message["jsonrpc"], "2.0");
            if message["method"] == "initialize" {
                assert_eq!(message["params"]["protocolVersion"], PROTOCOL_VERSION);
                let response = Json(json!({"jsonrpc":"2.0","id":1,"result":{
                    "protocolVersion":"2025-06-18","capabilities":{"tools":{}}
                }}));
                return ([("mcp-session-id", "fixture-session")], response).into_response();
            }
            assert_eq!(headers["mcp-session-id"], "fixture-session");
            assert_eq!(headers["mcp-protocol-version"], "2025-06-18");
            if message["method"] == "notifications/initialized" {
                assert!(message.get("id").is_none());
                return StatusCode::ACCEPTED.into_response();
            }
            assert_eq!(message, json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"estimate_cost","arguments":{"modelId":"seedance-20","inputs":{"duration":"5"}}
            }}));
            let result = json!({"jsonrpc":"2.0","id":2,"result":{
                "content":[{"type":"text","text":"{\"modelId\":\"seedance-20\",\"credits\":200}"}]
            }});
            ([("content-type", "text/event-stream")], format!(
                ": comment\r\ndata: {{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\"}}\r\n\r\ndata: {result}\r\n\r\n"
            )).into_response()
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/mcp", listener.local_addr().unwrap())).unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, server).await.unwrap() });
        let inputs = Map::from_iter([("duration".into(), json!("5"))]);
        let (result, sha256) = estimate(
            &Client::builder().no_proxy().build().unwrap(),
            &url,
            HeaderValue::from_static("Bearer fixture-only"),
            "seedance-20",
            &inputs,
        )
        .await
        .unwrap();
        task.abort();
        assert_eq!(result.credits.as_i64(), Some(200));
        assert_eq!(sha256.len(), 64);
    }
}
