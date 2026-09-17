use std::sync::Arc;

use axum::Router;

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

    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .expect("failed to bind port");
    print_banner(&config);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}

fn print_banner(config: &config::Config) {
    let host = &config.server.host;
    let port = config.server.port;
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
}
