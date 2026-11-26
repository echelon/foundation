use std::time::{Duration, Instant};

use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION, ETAG, IF_NONE_MATCH, RETRY_AFTER};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::trace::{OtlpTrace, RequestTraceSnapshot, TraceBaseline};
use crate::{
    valid_id, AudioProviderResult, Catalog, ChatOptions, Error, ImageProviderResult, ListOptions,
    MeshProviderResult, Message, ModelCatalog, PolicySource, Provider, ProviderPath, ProviderResult, Quote, QuoteInput, Receipt, RequestEvents,
    RequestFilters, RequestPage, RequestStatus, RequestTrace, RoutePreview, RoutePreviewOptions, SubmitOptions, UsageOptions, UsageReport,
    VideoProviderResult, WaitOptions,
};

#[derive(Clone)]
pub struct TokenCraftClient {
    origin: Url,
    bearer: Option<HeaderValue>,
    http: Client,
    app_ref: Option<String>,
}

impl TokenCraftClient {
    pub fn new(base_url: &str, api_key: Option<&str>) -> Result<Self, Error> {
        Self::with_timeout(base_url, api_key, Duration::from_secs(30))
    }

    pub fn with_timeout(
        base_url: &str,
        api_key: Option<&str>,
        timeout: Duration,
    ) -> Result<Self, Error> {
        validate_duration(timeout)?;
        let invalid_url = || {
            Error::InvalidInput(
                "use HTTPS or loopback HTTP without URL credentials, query or fragment",
            )
        };
        let mut origin = Url::parse(base_url).map_err(|_| invalid_url())?;
        let loopback = matches!(origin.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(origin.scheme() == "https" || (origin.scheme() == "http" && loopback))
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(invalid_url());
        }
        if !origin.path().ends_with('/') {
            origin
                .path_segments_mut()
                .map_err(|_| invalid_url())?
                .push("");
        }

        let bearer = api_key
            .map(|value| {
                let invalid = || Error::InvalidInput("supply a valid TokenCraft API key");
                if value.is_empty()
                    || value.len() > 512
                    || !value.bytes().all(|byte| byte.is_ascii_graphic())
                {
                    return Err(invalid());
                }
                let mut header =
                    HeaderValue::from_str(&format!("Bearer {value}")).map_err(|_| invalid())?;
                header.set_sensitive(true);
                Ok(header)
            })
            .transpose()?;

        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout);
        if loopback {
            builder = builder.no_proxy();
        }
        let http = builder
            .build()
            .map_err(|source| Error::from_http(source, None))?;
        Ok(Self {
            origin,
            bearer,
            http,
            app_ref: None,
        })
    }

    pub async fn catalog(&self) -> Result<Catalog, Error> {
        self.send(self.request(Method::GET, "v1/catalog/offers", false)?, None)
            .await
    }

    /// Discover models in the key's project, or the public catalog without a key.
    pub async fn models(&self) -> Result<ModelCatalog, Error> {
        match self.bearer {
            Some(_) => self.models_scoped().await,
            None => self.models_public().await,
        }
    }

    pub async fn models_public(&self) -> Result<ModelCatalog, Error> {
        self.send(self.request(Method::GET, "v1/catalog/models", false)?, None).await
    }

    /// Requires a key that can submit project requests.
    pub async fn models_scoped(&self) -> Result<ModelCatalog, Error> {
        self.send(self.request(Method::GET, "v1/catalog/models/scoped", true)?, None).await
    }

    pub async fn quote(&self, input: &QuoteInput) -> Result<Quote, Error> {
        match self.bearer {
            Some(_) => self.quote_scoped(input).await,
            None => self.quote_public(input).await,
        }
    }

    /// Request an ordered provider preference. Pass an empty list to select by lowest estimate.
    pub async fn quote_with_preferred_providers(
        &self,
        input: &QuoteInput,
        providers: &[Provider],
    ) -> Result<Quote, Error> {
        let scoped = self.bearer.is_some();
        let path = if scoped { "v1/quotes/scoped" } else { "v1/quotes" };
        let request = self.request(Method::POST, path, scoped)?.json(&PreferredQuoteInput {
            input,
            preferred_providers: Some(providers),
        });
        self.send(request, None).await
    }

    pub async fn quote_public(&self, input: &QuoteInput) -> Result<Quote, Error> {
        let request = self.request(Method::POST, "v1/quotes", false)?.json(input);
        self.send(request, None).await
    }

    pub async fn quote_scoped(&self, input: &QuoteInput) -> Result<Quote, Error> {
        let request = self
            .request(Method::POST, "v1/quotes/scoped", true)?
            .json(input);
        self.send(request, None).await
    }

    /// Quote using the same client attribution that will be sent with the request.
    pub async fn quote_scoped_for_client(
        &self,
        input: &QuoteInput,
        client_ref: &str,
    ) -> Result<Quote, Error> {
        let request = self
            .request(Method::POST, "v1/quotes/scoped", true)?
            .query(&[("client_ref", client_ref)])
            .json(input);
        self.send(request, None).await
    }

    /// Quote exactly one eligible gateway offer. Submitting the quote preserves the pin.
    pub async fn quote_offer(
        &self,
        input: &QuoteInput,
        offer_id: &str,
        client_ref: Option<&str>,
    ) -> Result<Quote, Error> {
        let mut request = self
            .request(Method::POST, "v1/quotes/scoped", true)?
            .query(&[("pinned_offer_id", offer_id)]);
        if let Some(client_ref) = client_ref {
            request = request.query(&[("client_ref", client_ref)]);
        }
        self.send(request.json(input), None).await
    }

    /// Set the application reference on submissions made by this client.
    pub fn with_app_ref(mut self, app_ref: impl Into<String>) -> Self {
        self.app_ref = Some(app_ref.into());
        self
    }

    pub async fn submit(
        &self,
        quote: &Quote,
        options: SubmitOptions<'_>,
    ) -> Result<Receipt, Error> {
        self.submit_request(quote, options, MediaPrompts::default())
            .await
    }

    pub async fn submit_video(
        &self,
        quote: &Quote,
        video_prompt: &str,
        options: SubmitOptions<'_>,
    ) -> Result<Receipt, Error> {
        self.submit_request(
            quote,
            options,
            MediaPrompts {
                video_prompt: Some(video_prompt),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn submit_audio(
        &self,
        quote: &Quote,
        audio_prompt: &str,
        options: SubmitOptions<'_>,
    ) -> Result<Receipt, Error> {
        self.submit_request(
            quote,
            options,
            MediaPrompts {
                audio_prompt: Some(audio_prompt),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn submit_mesh(
        &self,
        quote: &Quote,
        mesh_prompt: &str,
        options: SubmitOptions<'_>,
    ) -> Result<Receipt, Error> {
        self.submit_request(
            quote,
            options,
            MediaPrompts {
                mesh_prompt: Some(mesh_prompt),
                ..Default::default()
            },
        )
        .await
    }

    async fn submit_request(
        &self,
        quote: &Quote,
        options: SubmitOptions<'_>,
        prompts: MediaPrompts<'_>,
    ) -> Result<Receipt, Error> {
        let key = options.idempotency_key;
        if !(8..=128).contains(&key.len())
            || !key.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(Error::InvalidInput(
                "use 8 to 128 ASCII letters, numbers or . _ : - in Idempotency-Key",
            ));
        }
        let body = SubmitBody {
            quote_id: &quote.id,
            pinned_offer_id: quote.pinned_offer_id.as_deref(),
            input: PreferredQuoteInput {
                input: &quote.input,
                preferred_providers: quote.routing.as_ref()
                    .and_then(|routing| routing.provider_preference.as_ref())
                    .filter(|setting| setting.source == PolicySource::Request)
                    .map(|setting| setting.value.as_slice()),
            },
            client_ref: options.client_ref,
            app_ref: self.app_ref.as_deref(),
            delivery: options
                .webhook_subscription_id
                .map(|webhook_subscription_id| Delivery {
                    webhook_subscription_id,
                }),
            messages: options.messages,
            image_prompt: options.image_prompt,
            prompts,
            chat_options: options.chat_options,
        };
        let request = self
            .request(Method::POST, "v1/requests", true)?
            .header("Idempotency-Key", key)
            .json(&body);
        self.send(request, None).await
    }

    pub async fn get(&self, id: &str) -> Result<RequestTrace, Error> {
        let request = self.request_for(Method::GET, id, "")?;
        self.send(request, Some(id)).await
    }

    pub async fn events(&self, id: &str) -> Result<RequestEvents, Error> {
        let request = self.request_for(Method::GET, id, "/events")?;
        self.send(request, Some(id)).await
    }

    /// Read the request's recorded timing, logs, and provider attempts.
    pub async fn trace(&self, id: &str) -> Result<RequestTraceSnapshot, Error> {
        let request = self.request_for(Method::GET, id, "/trace")?;
        self.send(request, Some(id)).await
    }

    /// Download the request's redacted trace as an OTLP/HTTP JSON document.
    pub async fn trace_otlp(&self, id: &str) -> Result<OtlpTrace, Error> {
        let request = self.request_for(Method::GET, id, "/trace/otlp")?;
        self.send(request, Some(id)).await
    }

    /// Compare request timings with prior successful requests on the same route.
    pub async fn trace_baseline(&self, id: &str) -> Result<TraceBaseline, Error> {
        let request = self.request_for(Method::GET, id, "/trace/baseline")?;
        self.send(request, Some(id)).await
    }

    pub async fn provider_path(&self, id: &str) -> Result<Option<ProviderPath>, Error> {
        #[derive(serde::Deserialize)]
        struct Trace {
            provider_path: Option<ProviderPath>,
        }
        let request = self.request_for(Method::GET, id, "/trace")?;
        let trace: Trace = self.send(request, Some(id)).await?;
        Ok(trace.provider_path)
    }

    pub async fn preview_route(
        &self,
        id: &str,
        options: &RoutePreviewOptions,
    ) -> Result<RoutePreview, Error> {
        let request = self.request_for(Method::POST, id, "/route-preview")?.json(options);
        self.send(request, Some(id)).await
    }

    pub async fn result(&self, id: &str) -> Result<ProviderResult, Error> {
        let request = self.request_for(Method::GET, id, "/result")?;
        self.send(request, Some(id)).await
    }

    pub async fn image_result(&self, id: &str) -> Result<ImageProviderResult, Error> {
        let request = self.request_for(Method::GET, id, "/result")?;
        self.send(request, Some(id)).await
    }

    pub async fn video_result(&self, id: &str) -> Result<VideoProviderResult, Error> {
        let request = self.request_for(Method::GET, id, "/result")?;
        self.send(request, Some(id)).await
    }

    pub async fn audio_result(&self, id: &str) -> Result<AudioProviderResult, Error> {
        let request = self.request_for(Method::GET, id, "/result")?;
        self.send(request, Some(id)).await
    }

    pub async fn mesh_result(&self, id: &str) -> Result<MeshProviderResult, Error> {
        let request = self.request_for(Method::GET, id, "/result")?;
        self.send(request, Some(id)).await
    }

    pub async fn cancel(&self, id: &str) -> Result<RequestTrace, Error> {
        let request = self
            .request_for(Method::POST, id, "/cancel")?
            .json(&serde_json::json!({}));
        self.send(request, Some(id)).await
    }

    pub async fn list(&self, filters: ListOptions<'_>) -> Result<RequestPage, Error> {
        self.list_filtered(RequestFilters {
            client_ref: filters.client_ref,
            after: filters.after,
            ..Default::default()
        }).await
    }

    pub async fn list_filtered(&self, filters: RequestFilters<'_>) -> Result<RequestPage, Error> {
        let request = self
            .request(Method::GET, "v1/requests", true)?
            .query(&filters);
        self.send(request, None).await
    }

    pub async fn usage(&self, filters: UsageOptions<'_>) -> Result<UsageReport, Error> {
        self.usage_report(filters, None).await
    }

    pub async fn usage_for_app(
        &self,
        app_ref: &str,
        filters: UsageOptions<'_>,
    ) -> Result<UsageReport, Error> {
        self.usage_report(filters, Some(app_ref)).await
    }

    async fn usage_report(
        &self,
        filters: UsageOptions<'_>,
        app_ref: Option<&str>,
    ) -> Result<UsageReport, Error> {
        let request = self.request(Method::GET, "v1/usage", true)?.query(&filters);
        let request = match app_ref {
            Some(reference) => request.query(&[("app_ref", reference)]),
            None => request,
        };
        self.send(request, None).await
    }

    pub async fn wait(&self, id: &str, options: WaitOptions) -> Result<RequestTrace, Error> {
        validate_duration(options.timeout)?;
        validate_duration(options.poll_interval)?;
        tokio::time::timeout(options.timeout, async {
            let mut validator: Option<HeaderValue> = None;
            loop {
                let mut request = self.request_for(Method::GET, id, "")?;
                if let Some(tag) = &validator {
                    request = request.header(IF_NONE_MATCH, tag.clone());
                }
                let response = request.send().await
                    .map_err(|source| Error::from_http(source, Some(id.to_owned())))?;
                let suggested_delay = response.headers().get(RETRY_AFTER)
                    .and_then(|value| value.to_str().ok()?.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or_default();
                if response.status() != StatusCode::NOT_MODIFIED || validator.is_none() {
                    validator = response.headers().get(ETAG).cloned();
                    let trace: RequestTrace = Self::decode(response, Some(id)).await?;
                    if trace.status.is_terminal()
                        || trace.status == RequestStatus::AwaitingReconciliation
                    {
                        return Ok(trace);
                    }
                }
                tokio::time::sleep(options.poll_interval.max(suggested_delay).min(options.timeout)).await;
            }
        })
        .await
        .map_err(|_| Error::Timeout {
            request_id: Some(id.to_owned()),
        })?
    }

    pub(super) fn request_for(&self, method: Method, id: &str, suffix: &str) -> Result<RequestBuilder, Error> {
        if !valid_id(id, "req_") {
            return Err(Error::InvalidInput("supply a valid TokenCraft request ID"));
        }
        self.request(method, &format!("v1/requests/{id}{suffix}"), true)
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        authenticated: bool,
    ) -> Result<RequestBuilder, Error> {
        let url = self
            .origin
            .join(path)
            .map_err(|_| Error::InvalidInput("invalid gateway route"))?;
        let request = self
            .http
            .request(method, url)
            .header(ACCEPT, "application/json");
        if authenticated {
            Ok(request.header(
                AUTHORIZATION,
                self.bearer.as_ref().ok_or(Error::MissingKey)?.clone(),
            ))
        } else {
            Ok(request)
        }
    }

    pub(super) async fn send<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
        id_hint: Option<&str>,
    ) -> Result<T, Error> {
        let response = request
            .send()
            .await
            .map_err(|source| Error::from_http(source, id_hint.map(str::to_owned)))?;
        Self::decode(response, id_hint).await
    }

    async fn decode<T: DeserializeOwned>(
        response: Response,
        id_hint: Option<&str>,
    ) -> Result<T, Error> {
        let (response, request_id) = Self::checked_response(response, id_hint).await?;
        response.json().await.map_err(|source| Error::from_http(source, request_id))
    }

    pub(super) async fn checked_response(
        response: Response,
        id_hint: Option<&str>,
    ) -> Result<(Response, Option<String>), Error> {
        let request_id = ["tokencraft-request-id", "x-request-id"]
            .iter()
            .find_map(|name| {
                response
                    .headers()
                    .get(*name)
                    .and_then(|value| value.to_str().ok())
                    .filter(|value| valid_id(value, "req_"))
            })
            .or(id_hint)
            .map(str::to_owned);
        let status = response.status();
        if status.is_success() {
            return Ok((response, request_id));
        }
        let problem = match response.json::<ErrorEnvelope>().await {
            Ok(envelope) => envelope.error,
            Err(source) if source.is_decode() => Problem::default(),
            Err(source) => return Err(Error::from_http(source, request_id)),
        };
        Err(Error::Gateway {
            status,
            code: problem.code.unwrap_or_else(|| "http_error".into()),
            message: problem
                .message
                .unwrap_or_else(|| "the gateway did not return an error message".into()),
            request_id,
        })
    }
}

fn validate_duration(value: Duration) -> Result<(), Error> {
    if value.is_zero() || Instant::now().checked_add(value).is_none() {
        return Err(Error::InvalidInput(
            "supply a positive, representable timeout or polling interval",
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct PreferredQuoteInput<'a> {
    #[serde(flatten)]
    input: &'a QuoteInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferred_providers: Option<&'a [Provider]>,
}

#[derive(Serialize)]
struct SubmitBody<'a> {
    quote_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_offer_id: Option<&'a str>,
    input: PreferredQuoteInput<'a>,
    client_ref: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_ref: Option<&'a str>,
    delivery: Option<Delivery<'a>>,
    messages: Option<&'a [Message<'a>]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_prompt: Option<&'a str>,
    #[serde(flatten)]
    prompts: MediaPrompts<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_options: Option<&'a ChatOptions>,
}

#[derive(Default, Serialize)]
struct MediaPrompts<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    video_prompt: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_prompt: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mesh_prompt: Option<&'a str>,
}

#[derive(Serialize)]
struct Delivery<'a> {
    webhook_subscription_id: &'a str,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    #[serde(default)]
    error: Problem,
}

#[derive(Default, Deserialize)]
struct Problem {
    code: Option<String>,
    message: Option<String>,
}
