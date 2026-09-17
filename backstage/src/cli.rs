use crate::config::Config;

const USAGE: &str = "\
usage: panopticon [OPTIONS] [PORT]

  PORT                  监听端口（也可用环境变量 PORT）
  --host                监听 0.0.0.0
  --config <FILE>       加载 TOML 配置文件
  --print-config        打印最终生效的配置并退出（可用作模板）
  --help

命令行参数优先于配置文件。";

/// 解析命令行与环境变量，得到最终配置。`--help` / `--print-config` 会直接退出进程。
pub fn parse_config() -> Config {
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
