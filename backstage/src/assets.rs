//! 编译期内嵌的静态资源：前端页面、PWA 资源、示例 TOML。

use axum::{
    Router,
    http::header,
    response::{Html, IntoResponse},
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
    app
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
