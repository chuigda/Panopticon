use axum::{
    Router, body::Body, extract::{Request, State}, http::{HeaderName, StatusCode, header}, response::{Html, Response}, routing::{get, post},
};

const UPSTREAM_HEADER: &str = "x-upstream-base-url";
const INDEX_HTML: &str = include_str!("../../frontend/dist/index.html");
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

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

    let app = Router::new()
        .route("/", get(index))
        .route("/v1/messages", post(proxy))
        .route("/v1/chat/completions", post(proxy))
        .route("/examples", get(index_examples))
        .merge(examples_router())
        .with_state(reqwest::Client::new());

    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .expect("failed to bind port");
    println!("Listening on http://{host}:{port}");
    println!("  - Access:        http://127.0.0.1:{port}");
    println!("  - Force Desktop: http://127.0.0.1:{port}/?desktop");
    println!("  - Force Mobile:  http://127.0.0.1:{port}/?mobile");

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
