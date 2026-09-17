use axum::{
    Router, body::Body, extract::{Request, State}, http::{HeaderName, StatusCode, header}, response::{Html, Response}, routing::{get, post},
};

const UPSTREAM_HEADER: &str = "x-upstream-base-url";
const INDEX_HTML: &str = include_str!("../../frontend/dist/index.html");
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

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

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let host = if args.iter().any(|arg| arg == "--host") {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };

    let port: u16 = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .or_else(|| std::env::var("PORT").ok())
        .map(|s| s.parse().expect("invalid port"))
        .unwrap_or(3000);

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
    let app = app.with_state(reqwest::Client::new());

    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .expect("failed to bind port");
    println!("Listening on http://{host}:{port}");
    if host == "0.0.0.0" {
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
    State(client): State<reqwest::Client>,
    req: Request,
) -> Result<Response, (StatusCode, String)> {
    let (parts, body) = req.into_parts();

    let base = parts
        .headers
        .get(UPSTREAM_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("missing or invalid `{UPSTREAM_HEADER}` header"),
            )
        })?;

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", base.trim_end_matches('/'), path_and_query);

    let body = axum::body::to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("failed to read body: {e}")))?;

    let mut upstream_req = client.request(parts.method, &url).body(body);
    for (name, value) in &parts.headers {
        if forwards_to_upstream(name) {
            upstream_req = upstream_req.header(name, value);
        }
    }

    let upstream_resp = upstream_req
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream request failed: {e}")))?;

    let mut builder = Response::builder().status(upstream_resp.status());
    for (name, value) in upstream_resp.headers() {
        if !is_hop_by_hop(name) {
            builder = builder.header(name, value);
        }
    }
    // 流式透传响应体,SSE 逐块转发
    Ok(builder
        .body(Body::from_stream(upstream_resp.bytes_stream()))
        .expect("failed to build response"))
}

fn forwards_to_upstream(name: &HeaderName) -> bool {
    name != header::HOST && name != header::CONTENT_LENGTH && name != UPSTREAM_HEADER && !is_hop_by_hop(name)
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
