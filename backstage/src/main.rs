use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Router, body::Body, extract::{Request, State}, http::{HeaderMap, HeaderName, StatusCode, header}, response::{Html, Response}, routing::{get, post},
};

mod config;
use config::Config;

const UPSTREAM_HEADER: &str = "x-upstream-base-url";
const INDEX_HTML: &str = include_str!("../../frontend/dist/index.html");

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    config: Arc<Config>,
    // 已通过 /v1/models 校验的上游 base URL 及校验时间
    verified: Arc<Mutex<HashMap<String, Instant>>>,
}

// PWA 静态资源，来自 frontend/public（sw.js 取构建后带 BUILD_ID 的版本）
const PWA_ASSETS: &[(&str, &str, &[u8])] = &[
    ("/manifest.webmanifest", "application/manifest+json", include_bytes!("../../frontend/dist/manifest.webmanifest")),
    ("/sw.js", "application/javascript", include_bytes!("../../frontend/dist/sw.js")),
    ("/.well-known/assetlinks.json", "application/json", include_bytes!("../../frontend/dist/.well-known/assetlinks.json")),
    ("/icons/icon-192.png", "image/png", include_bytes!("../../frontend/dist/icons/icon-192.png")),
    ("/icons/icon-512.png", "image/png", include_bytes!("../../frontend/dist/icons/icon-512.png")),
    ("/icons/icon-192-maskable.png", "image/png", include_bytes!("../../frontend/dist/icons/icon-192-maskable.png")),
    ("/icons/icon-512-maskable.png", "image/png", include_bytes!("../../frontend/dist/icons/icon-512-maskable.png")),
];

const USAGE: &str = "\
usage: panopticon [OPTIONS] [PORT]

  PORT                  监听端口（也可用环境变量 PORT）
  --host                监听 0.0.0.0
  --config <FILE>       加载 TOML 配置文件
  --print-config        打印最终生效的配置并退出（可用作模板）
  --help

命令行参数优先于配置文件。";

fn parse_config() -> Config {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        std::process::exit(0);
    }

    let mut cfg = match args.iter().position(|a| a == "--config") {
        Some(i) => {
            let path = args.get(i + 1).unwrap_or_else(|| {
                eprintln!("--config requires a file path");
                std::process::exit(2);
            });
            Config::load(path).unwrap_or_else(|e| {
                eprintln!("failed to load config: {e}");
                std::process::exit(2);
            })
        }
        None => Config::default(),
    };

    if args.iter().any(|a| a == "--host") {
        cfg.server.host = "0.0.0.0".into();
    }
    let mut skip_next = false;
    let port_arg = args.iter().find(|a| {
        if skip_next {
            skip_next = false;
            return false;
        }
        if *a == "--config" {
            skip_next = true;
            return false;
        }
        !a.starts_with("--")
    });
    if let Some(p) = port_arg.cloned().or_else(|| std::env::var("PORT").ok()) {
        cfg.server.port = p.parse().expect("invalid port");
    }

    if args.iter().any(|a| a == "--print-config") {
        println!("# panopticon 配置。省略的字段使用默认值。");
        println!("# [upstream] allow_private_network 未设置时：监听回环地址则为 true，否则为 false。");
        println!("# 当前生效值：{}\n", cfg.allow_private_network());
        print!("{}", toml::to_string_pretty(&cfg).expect("serialize config"));
        std::process::exit(0);
    }
    cfg
}

#[tokio::main]
async fn main() {
    let config = Arc::new(parse_config());
    let host = config.server.host.as_str();
    let port = config.server.port;

    let mut app = Router::new()
        .route("/", get(index))
        .route("/v1/messages", post(proxy))
        .route("/v1/chat/completions", post(proxy))
        .route("/examples", get(index_examples))
        .route("/examples/index.json", get(index_examples))
        .merge(examples_router());
    for (route, mime, bytes) in PWA_ASSETS {
        app = app.route(route, get(move || async move { ([(header::CONTENT_TYPE, *mime)], *bytes) }));
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(config.connect_timeout())
        .read_timeout(config.stream_idle_timeout())
        .build()
        .expect("failed to build http client");
    let app = app.with_state(AppState {
        client,
        config: config.clone(),
        verified: Arc::new(Mutex::new(HashMap::new())),
    });

    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .expect("failed to bind port");
    println!("Listening on http://{host}:{port}");
    if config.allow_private_network() {
        println!("\t! private-network upstreams are ALLOWED");
    }
    if !config.listens_on_loopback() {
        if let Ok(ifaces) = local_ip_address::list_afinet_netifas() {
            for (name, ip) in ifaces {
                let url = match ip {
                    std::net::IpAddr::V4(v4) => format!("http://{v4}:{port}"),
                    std::net::IpAddr::V6(v6) => format!("http://[{v6}]:{port}"),
                };
                println!("\t- {name}\t\t\t{url}");
            }
        }
    } else {
        println!("\t- Access:\t\thttp://127.0.0.1:{port}");
    }

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

include!("examples.rs");

async fn index_examples() -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("examples.json"),
    )
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
