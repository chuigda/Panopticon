use std::{sync::Arc, time::Duration};

use axum::Router;
use axum_server::tls_rustls::RustlsConfig;

mod assets;
mod cli;
mod config;
mod gatchpt;
mod proxy;

#[tokio::main]
async fn main() {
    let config = Arc::new(cli::parse_config());
    let host = config.server.host.as_str();
    let port = config.server.port;

    let app = Router::new()
        .merge(assets::router())
        .merge(gatchpt::router())
        .merge(proxy::router(config.clone()));

    match &config.server.tls {
        None => {
            let listener = tokio::net::TcpListener::bind((host, port))
                .await
                .expect("failed to bind port");
            print_banner(&config);
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        }
        Some(tls) => {
            rustls::crypto::ring::default_provider()
                .install_default()
                .ok();
            let tls_config = RustlsConfig::from_pem_file(&tls.cert_file, &tls.key_file)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("failed to load TLS cert/key: {e}");
                    std::process::exit(2);
                });
            let listener = std::net::TcpListener::bind((host, port)).expect("failed to bind port");
            print_banner(&config);

            let handle = axum_server::Handle::new();
            tokio::spawn({
                let handle = handle.clone();
                async move {
                    shutdown_signal().await;
                    handle.graceful_shutdown(Some(Duration::from_secs(10)));
                }
            });
            axum_server::from_tcp_rustls(listener, tls_config)
                .handle(handle)
                .serve(app.into_make_service())
                .await
                .unwrap();
        }
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}

fn print_banner(config: &config::Config) {
    let host = &config.server.host;
    let port = config.server.port;
    let scheme = config.scheme();
    println!("Listening on {scheme}://{host}:{port}");
    if config.allow_private_network() {
        println!("\t! private-network upstreams are ALLOWED");
    }
    if !config.listens_on_loopback() {
        if let Ok(ifaces) = local_ip_address::list_afinet_netifas() {
            for (name, ip) in ifaces {
                let url = match ip {
                    std::net::IpAddr::V4(v4) => format!("{scheme}://{v4}:{port}"),
                    std::net::IpAddr::V6(v6) => format!("{scheme}://[{v6}]:{port}"),
                };
                println!("\t- {name}\t\t\t{url}");
            }
        }
    } else {
        println!("\t- Access:\t\t{scheme}://127.0.0.1:{port}");
    }
}
