//! LLM API 反向代理：按 `x-upstream-base-url` 头转发到上游，并做 SSRF 防护与上游校验。

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, StatusCode, header},
    response::Response,
    routing::post,
};

use crate::config::Config;

const UPSTREAM_HEADER: &str = "x-upstream-base-url";

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    config: Arc<Config>,
    // 已通过 /v1/models 校验的上游 base URL 及校验时间
    verified: Arc<Mutex<HashMap<String, Instant>>>,
}

pub fn router(config: Arc<Config>) -> Router {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(config.connect_timeout())
        .read_timeout(config.stream_idle_timeout())
        .build()
        .expect("failed to build http client");
    Router::new()
        .route("/v1/messages", post(proxy))
        .route("/v1/chat/completions", post(proxy))
        .with_state(AppState {
            client,
            config,
            verified: Arc::new(Mutex::new(HashMap::new())),
        })
}

async fn proxy(
    State(state): State<AppState>,
    req: Request,
) -> Result<Response, (StatusCode, String)> {
    let (parts, body) = req.into_parts();

    let base = parts
        .headers
        .get(UPSTREAM_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_end_matches('/'))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("missing or invalid `{UPSTREAM_HEADER}` header"),
            )
        })?;

    verify_upstream(&state, base, &parts.headers).await?;
    let client = &state.client;
    let cfg = &state.config;

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    if has_traversal(path_and_query) {
        return Err((StatusCode::BAD_REQUEST, "invalid path".into()));
    }
    let url = format!("{base}{path_and_query}");

    let body = tokio::time::timeout(
        cfg.body_read_timeout(),
        axum::body::to_bytes(body, cfg.limits.max_body_bytes),
    )
    .await
    .map_err(|_| (StatusCode::REQUEST_TIMEOUT, "request body read timed out".to_string()))?
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("failed to read body: {e}")))?;

    let mut upstream_req = client.request(parts.method, &url).body(body);
    for (name, value) in &parts.headers {
        if forwards_to_upstream(name) && cfg.forwards_request_header(name) {
            upstream_req = upstream_req.header(name, value);
        }
    }

    let upstream_resp = tokio::time::timeout(cfg.upstream_headers_timeout(), upstream_req.send())
        .await
        .map_err(|_| (StatusCode::GATEWAY_TIMEOUT, "upstream did not respond in time".to_string()))?
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream request failed: {e}")))?;

    let mut builder = Response::builder().status(upstream_resp.status());
    for (name, value) in upstream_resp.headers() {
        if !is_hop_by_hop(name) && cfg.forwards_response_header(name) {
            builder = builder.header(name, value);
        }
    }
    // 流式透传响应体,SSE 逐块转发
    Ok(builder
        .body(Body::from_stream(upstream_resp.bytes_stream()))
        .expect("failed to build response"))
}

/// 通过 GET {base}/v1/models 确认上游是 OpenAI 兼容 / Anthropic 风格的 LLM API，结果按 base 缓存。
async fn verify_upstream(
    state: &AppState,
    base: &str,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    let reject = |msg: &str| (StatusCode::FORBIDDEN, format!("upstream rejected: {msg}"));
    let cfg = &state.config;

    if has_traversal(base) {
        return Err(reject("invalid base url"));
    }
    if cfg.is_trusted(base) {
        return Ok(());
    }
    if cfg.upstream.trusted_only {
        return Err(reject("not in trusted list"));
    }

    let url = reqwest::Url::parse(base).map_err(|_| reject("invalid base url"))?;
    if url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(reject("invalid base url"));
    }
    let host = url.host_str().ok_or_else(|| reject("invalid base url"))?;
    let allow_private = cfg.allow_private_network();
    let allow_loopback = cfg.upstream.allow_loopback;
    let loopback_host = host == "localhost"
        || host.trim_matches(['[', ']']).parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    match url.scheme() {
        "https" => {}
        "http" if allow_private || (allow_loopback && loopback_host) => {}
        _ => return Err(reject("https required")),
    }

    {
        let mut cache = state.verified.lock().unwrap();
        cache.retain(|_, t| t.elapsed() < cfg.verify_ttl());
        if cache.contains_key(base) {
            return Ok(());
        }
    }

    if !allow_private {
        let port = url.port_or_known_default().unwrap_or(443);
        let addrs = tokio::net::lookup_host((host.trim_matches(['[', ']']), port))
            .await
            .map_err(|_| reject("dns resolution failed"))?;
        let mut any = false;
        for addr in addrs {
            any = true;
            if allow_loopback && addr.ip().is_loopback() {
                continue;
            }
            if is_forbidden_ip(addr.ip()) {
                return Err(reject("private or reserved address"));
            }
        }
        if !any {
            return Err(reject("dns resolution failed"));
        }
    }

    if !cfg.upstream.verify_models_endpoint {
        state.verified.lock().unwrap().insert(base.to_owned(), Instant::now());
        return Ok(());
    }

    let mut probe = state
        .client
        .get(format!("{base}/v1/models"))
        .timeout(cfg.verify_timeout());
    for name in [header::AUTHORIZATION, HeaderName::from_static("x-api-key"), HeaderName::from_static("anthropic-version")] {
        if let Some(v) = headers.get(&name) {
            probe = probe.header(name, v);
        }
    }

    let resp = probe.send().await.map_err(|_| reject("models endpoint unreachable"))?;
    if !resp.status().is_success() {
        return Err(reject("models endpoint returned non-2xx"));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| reject("failed to read models response"))?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| reject("models endpoint returned non-json"))?;
    let looks_like_model_list = json
        .get("data")
        .and_then(|d| d.as_array())
        .is_some_and(|arr| !arr.is_empty() && arr.iter().all(|m| m.get("id").is_some_and(|id| id.is_string())));
    if !looks_like_model_list {
        return Err(reject("models endpoint response not recognized"));
    }

    state.verified.lock().unwrap().insert(base.to_owned(), Instant::now());
    Ok(())
}

fn has_traversal(s: &str) -> bool {
    s.contains("..") || s.contains('\\') || s.contains("%2e") || s.contains("%2E") || s.contains("%2f") || s.contains("%2F")
}

/// 回环、私网、链路本地（含云元数据 169.254.x.x）、CGNAT、组播、未指定等非公网地址。
fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
                || v4.octets()[0] == 0
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_forbidden_ip(IpAddr::V4(v4));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || (v6.segments()[0] & 0xffff) == 0x2001 && v6.segments()[1] == 0x0db8
        }
    }
}

/// 无论配置如何都不转发的请求头。
fn forwards_to_upstream(name: &HeaderName) -> bool {
    name != header::HOST
        && name != header::CONTENT_LENGTH
        && name != UPSTREAM_HEADER
        && !is_hop_by_hop(name)
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    name == header::CONNECTION
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
        || name == header::TE
        || name == header::TRAILER
        || name == header::PROXY_AUTHENTICATE
        || name == header::PROXY_AUTHORIZATION
        || name == "keep-alive"
}
