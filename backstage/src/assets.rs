//! 编译期内嵌的静态资源：前端页面、PWA 资源、示例 TOML。

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};

const INDEX_HTML: &str = include_str!("../../frontend/dist/index.html");

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

include!("examples.rs");

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let mut app = Router::new()
        .route("/", get(index))
        .route("/examples", get(index_examples))
        .route("/examples/index.json", get(index_examples))
        .merge(examples_router());
    for (route, mime, bytes) in PWA_ASSETS {
        app = app.route(route, get(move || async move { ([(header::CONTENT_TYPE, *mime)], *bytes) }));
    }
    app.layer(middleware::from_fn(security_headers))
}

// 前端为 single-file 构建，脚本/样式内联，故 script/style 需 'unsafe-inline'；
// 直连模式下浏览器会 fetch 用户填写的任意 https 端点，故 connect-src 放开 https:。
// 先以 Report-Only 观察，无误伤后再切换为 Content-Security-Policy。
const CSP: &str = "default-src 'self'; \
    script-src 'self' 'unsafe-inline'; \
    style-src 'self' 'unsafe-inline'; \
    connect-src 'self' https:; \
    img-src 'self' data:; \
    font-src 'self' data:; \
    object-src 'none'; \
    base-uri 'self'; \
    frame-ancestors 'none'; \
    form-action 'self'; \
    worker-src 'self'";

const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("content-security-policy-report-only", CSP),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "no-referrer"),
    ("permissions-policy", "camera=(), microphone=(), geolocation=()"),
    ("cross-origin-opener-policy", "same-origin"),
];

async fn security_headers(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    for (name, value) in SECURITY_HEADERS {
        headers.insert(*name, HeaderValue::from_static(value));
    }
    resp
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn index_examples() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("examples.json"),
    )
}
