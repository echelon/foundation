use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode};
use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use db_queries::gateway_requests::chat_streams;
use serde_json::{json, Value};
use tokio::sync::{mpsc, OwnedSemaphorePermit};
use tokio_stream::wrappers::ReceiverStream;

use super::{
    access, bearer, header, request_headers, requests, responses, supported_finish_reason,
    AcceptedRequest, ChatError, Permission, RequestStatus, ScopeQuery, Wire, CLIENT_REF,
    X_SHOULD_RETRY,
};
use crate::error::ApiError;
use crate::gateway_demo::providers::{stream_chunk_purpose, ProviderRuntime};
use crate::state::AppState;

const WAIT: Duration = Duration::from_secs(45);
type Sender = mpsc::Sender<Result<Event, Infallible>>;

pub(super) fn response(
    state: AppState,
    runtime: Arc<ProviderRuntime>,
    accepted: AcceptedRequest,
    model: String,
    headers: HeaderMap,
    wire: Wire,
    permit: OwnedSemaphorePermit,
) -> Response {
    let mut response = request_headers(&accepted);
    response.insert(X_SHOULD_RETRY, HeaderValue::from_static("false"));
    response.insert("x-accel-buffering", HeaderValue::from_static("no"));
    let mut auth = HeaderMap::new();
    if let Some(mut token) = headers.get(AUTHORIZATION).cloned() {
        token.set_sensitive(true);
        auth.insert(AUTHORIZATION, token);
    }
    if let Some(client) = headers.get(CLIENT_REF).cloned() {
        auth.insert(CLIENT_REF, client);
    }
    let reader = Reader {
        state,
        runtime,
        accepted,
        model,
        auth,
        wire,
    };
    let (send, receive) = mpsc::channel(8);
    tokio::spawn(async move {
        let _permit = permit;
        let mut responses_stream = responses::Stream::default();
        let outcome = tokio::select! {
            result = tokio::time::timeout(WAIT, reader.pump(&send, &mut responses_stream)) => Some(result),
            _ = send.closed() => None,
        };
        if outcome.is_none() || send.is_closed() {
            if let Err(error) =
                chat_streams::record_detach(&reader.state.pool, &reader.accepted.id).await
            {
                tracing::warn!(sqlstate = ?error.as_database_error().and_then(|error| error.code()),
                    "could not record chat stream detachment");
            }
            return;
        }
        let error = match outcome {
            Some(Ok(Ok(()))) => return,
            Some(Ok(Err(error))) => error,
            _ => ChatError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "request_pending",
                "The request is still running. Follow Location to poll its status.",
            ),
        };
        let event = match wire {
            Wire::Chat { .. } => Event::default().data(error.stream_data()),
            Wire::Responses(_) => responses_stream.error(error),
        };
        let _ = tokio::time::timeout(Duration::from_secs(1), send.send(Ok(event))).await;
    });
    (
        response,
        Sse::new(ReceiverStream::new(receive)).keep_alive(KeepAlive::default()),
    )
        .into_response()
}

struct Reader {
    state: AppState,
    runtime: Arc<ProviderRuntime>,
    accepted: AcceptedRequest,
    model: String,
    auth: HeaderMap,
    wire: Wire,
}

impl Reader {
    async fn pump(
        &self,
        send: &Sender,
        responses_stream: &mut responses::Stream,
    ) -> Result<(), ChatError> {
        let id = &self.accepted.id;
        let client = header(&self.auth, CLIENT_REF)?;
        let mut after = 0;
        let mut sent_bytes = 0;
        let mut response_started = false;
        let mut item_started = false;
        let mut delay = Duration::from_millis(100);
        loop {
            let caller = bearer(&self.auth, &self.state).await?;
            let access = access::resolve(
                &self.state,
                caller,
                ScopeQuery::default(),
                Permission::ReadRequests,
            )
            .await?;
            let scope = access.for_client(client)?;
            let page = chat_streams::read(&self.state.pool, scope, id, after)
                .await
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NotFound("request not found".into()))?;
            if page.content_removed_at.is_some() {
                return Err(ChatError::new(
                    StatusCode::GONE,
                    "result_removed",
                    "Stored completion content was removed.",
                ));
            }
            let created = page.created_at.timestamp();
            let lifecycle = match self.wire {
                Wire::Chat { .. } => None,
                Wire::Responses(settings) => Some(responses::TextLifecycle {
                    request: id.as_str(),
                    model: &self.model,
                    created,
                    settings,
                }),
            };
            if let Some(lifecycle) = lifecycle.as_ref().filter(|_| !response_started) {
                if !emit_all(send, responses_stream.created(lifecycle)).await {
                    return Ok(());
                }
                response_started = true;
            }
            if !item_started && (!page.chunks.is_empty() || page.status == RequestStatus::Succeeded) {
                let sent = match &lifecycle {
                    Some(lifecycle) => emit_all(send, responses_stream.start_item(lifecycle)).await,
                    None => {
                        self.emit_chat(
                            send,
                            created,
                            json!([choice(json!({"role":"assistant"}), Value::Null)]),
                            None,
                        )
                        .await
                    }
                };
                if !sent {
                    return Ok(());
                }
                item_started = true;
            }
            let more = page.chunks.len() == chat_streams::PAGE_SIZE;
            for chunk in page.chunks {
                let text = self
                    .runtime
                    .payloads
                    .open(
                        id.as_str(),
                        &stream_chunk_purpose(chunk.ordinal),
                        &chunk.payload,
                    )
                    .and_then(|body| String::from_utf8(body).ok())
                    .ok_or_else(stream_unavailable)?;
                if chunk.ordinal != after + 1 {
                    return Err(stream_unavailable());
                }
                after = chunk.ordinal;
                sent_bytes += text.len();
                let sent = match &lifecycle {
                    Some(lifecycle) => {
                        emit_all(send, [responses_stream.delta(lifecycle, text)]).await
                    }
                    None => {
                        self.emit_chat(
                            send,
                            created,
                            json!([choice(json!({"content":text}), Value::Null)]),
                            None,
                        )
                        .await
                    }
                };
                if !sent {
                    return Ok(());
                }
                delay = Duration::from_millis(100);
            }
            if more {
                continue;
            }
            match page.status {
                RequestStatus::Queued | RequestStatus::Running => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_millis(400));
                }
                RequestStatus::Succeeded => {
                    let result =
                        requests::provider_result(&self.state.pool, Some(&self.runtime), scope, id)
                            .await
                            .map_err(|error| match error {
                                ApiError::NotFound(_) => ChatError::new(
                                    StatusCode::GONE,
                                    "result_expired",
                                    "The completion text has expired.",
                                ),
                                other => other.into(),
                            })?
                            .result;
                    if result.text.len() != sent_bytes {
                        return Err(stream_unavailable());
                    }
                    if let Some(lifecycle) = &lifecycle {
                        let reason = supported_finish_reason(result.finish_reason.as_deref())?;
                        let completed_at = page.succeeded_at.map_or(created, |at| at.timestamp());
                        let response = lifecycle.terminal(
                            result.text,
                            reason,
                            completed_at,
                            page.token_counts,
                        );
                        emit_all(send, responses_stream.finish(response)).await;
                        return Ok(());
                    }
                    if !self
                        .emit_chat(
                            send,
                            created,
                            json!([choice(json!({}), json!(result.finish_reason))]),
                            None,
                        )
                        .await
                    {
                        return Ok(());
                    }
                    if matches!(
                        self.wire,
                        Wire::Chat {
                            include_usage: true
                        }
                    ) {
                        if let Some((prompt, completion)) = page.token_counts {
                            let usage = json!({"prompt_tokens": prompt, "completion_tokens": completion,
                                "total_tokens": i64::from(prompt) + i64::from(completion)});
                            if !self.emit_chat(send, created, json!([]), Some(usage)).await {
                                return Ok(());
                            }
                        }
                    }
                    let _ = send.send(Ok(Event::default().data("[DONE]"))).await;
                    return Ok(());
                }
                RequestStatus::AwaitingReconciliation => return Err(ChatError::new(
                    StatusCode::BAD_GATEWAY,
                    "provider_outcome_unconfirmed",
                    "The provider outcome is unconfirmed. Follow Location to inspect this request.",
                )),
                RequestStatus::Failed => {
                    return Err(ChatError::new(
                        StatusCode::BAD_GATEWAY,
                        "provider_failed",
                        "The provider request failed. Follow Location to inspect this request.",
                    ))
                }
                RequestStatus::Canceled => {
                    return Err(ChatError::new(
                        StatusCode::CONFLICT,
                        "request_canceled",
                        "The request was canceled.",
                    ))
                }
            }
        }
    }

    async fn emit_chat(
        &self,
        send: &Sender,
        created: i64,
        choices: Value,
        usage: Option<Value>,
    ) -> bool {
        let mut payload = json!({
            "id": self.accepted.id, "object": "chat.completion.chunk",
            "created": created, "model": self.model, "choices": choices
        });
        if matches!(
            self.wire,
            Wire::Chat {
                include_usage: true
            }
        ) {
            payload["usage"] = usage.unwrap_or(Value::Null);
        }
        send.send(Ok(Event::default().data(payload.to_string())))
            .await
            .is_ok()
    }
}

async fn emit_all<const N: usize>(send: &Sender, events: [Event; N]) -> bool {
    for event in events {
        if send.send(Ok(event)).await.is_err() {
            return false;
        }
    }
    true
}

fn choice(delta: Value, finish_reason: Value) -> Value {
    json!({"index": 0, "delta": delta, "finish_reason": finish_reason})
}

fn stream_unavailable() -> ChatError {
    ChatError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "stream_unavailable",
        "Stored stream data is unavailable. Follow Location to inspect this request.",
    )
}
