// 由 gen-examples.sh 自动生成，请勿手动修改

fn examples_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(
            "/examples/fate/addon/hgw4.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/fate/addon/hgw4.addon.chr.toml")) }),
        )
        .route(
            "/examples/fate/addon/hgw5.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/fate/addon/hgw5.addon.chr.toml")) }),
        )
        .route(
            "/examples/fate/fate.simulator.cd.chr.toml",
            get(|| async { toml_response(include_str!("../../example/fate/fate.simulator.cd.chr.toml")) }),
        )
        .route(
            "/examples/fate/fate.simulator.mk.chr.toml",
            get(|| async { toml_response(include_str!("../../example/fate/fate.simulator.mk.chr.toml")) }),
        )
        .route(
            "/examples/genshin/genshin.simulator.cd.chr.toml",
            get(|| async { toml_response(include_str!("../../example/genshin/genshin.simulator.cd.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/eastern-magic.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/eastern-magic.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/goblet-of-fire.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/goblet-of-fire.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/gringotts.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/gringotts.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/hachimi.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/hachimi.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/lovegood-ish.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/lovegood-ish.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/magecraft.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/magecraft.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/magic.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/magic.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/addon/spells.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/addon/spells.addon.chr.toml")) }),
        )
        .route(
            "/examples/hp/harry-potter.simulator.cd.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/harry-potter.simulator.cd.chr.toml")) }),
        )
        .route(
            "/examples/hp/harry-potter.simulator.mk.chr.toml",
            get(|| async { toml_response(include_str!("../../example/hp/harry-potter.simulator.mk.chr.toml")) }),
        )
        .route(
            "/examples/medieval/addon/telephone.addon.chr.toml",
            get(|| async { toml_response(include_str!("../../example/medieval/addon/telephone.addon.chr.toml")) }),
        )
        .route(
            "/examples/medieval/medieval.simulator.cd.chr.toml",
            get(|| async { toml_response(include_str!("../../example/medieval/medieval.simulator.cd.chr.toml")) }),
        )
        .route(
            "/examples/player/anton.player.chr.toml",
            get(|| async { toml_response(include_str!("../../example/player/anton.player.chr.toml")) }),
        )
}

fn toml_response(body: &'static str) -> impl axum::response::IntoResponse {
    ([(header::CONTENT_TYPE, "application/toml")], body)
}
