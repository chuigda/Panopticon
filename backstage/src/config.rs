use std::{net::IpAddr, time::Duration};

use axum::http::HeaderName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: Server,
    pub upstream: Upstream,
    pub headers: Headers,
    pub limits: Limits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    /// 监听地址，`0.0.0.0` 为公网模式
    pub host: String,
    pub port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Self { host: "127.0.0.1".into(), port: 3000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Upstream {
    /// 允许上游为 http:// 以及回环 / 私网 / 链路本地地址。
    /// 未设置时：监听回环地址则允许，否则拒绝。
    pub allow_private_network: Option<bool>,
    /// 信任的上游 base URL（无尾部 `/`），跳过 /v1/models 校验
    pub trusted: Vec<String>,
    /// 仅允许 `trusted` 中的上游
    pub trusted_only: bool,
    /// 对非信任上游通过 GET /v1/models 校验其为 LLM API
    pub verify_models_endpoint: bool,
    /// 校验结果缓存秒数
    pub verify_ttl_secs: u64,
}

impl Default for Upstream {
    fn default() -> Self {
        Self {
            allow_private_network: None,
            trusted: [
                // --- 先进国际模型 ---

                // OpenAI
                "https://api.openai.com",
                // Anthropic
                "https://api.anthropic.com",
                // X AI (Grok)
                "https://api.x.ai",
                // Vercel
                "https://ai-gateway.vercel.sh",
                // 点名批评：Google Deepmind

                // --- 中国模型 ---

                // 深度求索
                "https://api.deepseek.com",
                // 月之暗面
                "https://api.moonshot.cn",
                // MiniMax (国内版 / 海外版)
                "https://api.minimaxi.com",
                "https://api.minimax.io",
                // 小米 Mimo
                "https://api.xiaomimimo.com",
                // 阿里云百炼
                "https://dashscope.aliyuncs.com",
                // 点名批评：火山引擎、字节跳动、智谱清言

                // --- 中转站 ---

                // OpenRouter
                "https://openrouter.ai/api",
                // 302.AI
                "https://api.302.ai",
            ]
            .map(String::from)
            .to_vec(),
            trusted_only: false,
            verify_models_endpoint: true,
            verify_ttl_secs: 3600,
        }
    }
}

/// 头名匹配规则：小写精确匹配，或以 `*` 结尾表示前缀匹配。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Headers {
    /// 非空则为白名单模式：仅转发列出的请求头
    pub request_allow: Vec<String>,
    /// 不转发给上游的请求头
    pub request_deny: Vec<String>,
    /// 非空则为白名单模式：仅回传列出的响应头
    pub response_allow: Vec<String>,
    /// 不回传给客户端的响应头
    pub response_deny: Vec<String>,
}

impl Default for Headers {
    fn default() -> Self {
        Self {
            request_allow: vec![],
            request_deny: [
                "cookie", "origin", "referer", "via", "forwarded",
                "x-forwarded-*", "x-real-ip", "cf-*", "fly-*", "sec-*",
            ]
            .map(String::from)
            .to_vec(),
            response_allow: vec![],
            response_deny: vec!["set-cookie".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_body_bytes: usize,
    pub connect_timeout_secs: u64,
    /// 读取客户端请求体的最长时间
    pub body_read_timeout_secs: u64,
    /// 等待上游返回响应头的最长时间（含推理排队）
    pub upstream_headers_timeout_secs: u64,
    /// SSE 流中相邻两块数据的最长间隔
    pub stream_idle_timeout_secs: u64,
    /// /v1/models 校验请求的整体超时
    pub verify_timeout_secs: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_body_bytes: 8 * 1024 * 1024,
            connect_timeout_secs: 10,
            body_read_timeout_secs: 30,
            upstream_headers_timeout_secs: 120,
            stream_idle_timeout_secs: 90,
            verify_timeout_secs: 10,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let mut cfg: Config = toml::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        cfg.normalize();
        Ok(cfg)
    }

    pub fn normalize(&mut self) {
        for t in &mut self.upstream.trusted {
            *t = t.trim_end_matches('/').to_owned();
        }
        for list in [
            &mut self.headers.request_allow,
            &mut self.headers.request_deny,
            &mut self.headers.response_allow,
            &mut self.headers.response_deny,
        ] {
            for h in list.iter_mut() {
                *h = h.to_ascii_lowercase();
            }
        }
    }

    pub fn listens_on_loopback(&self) -> bool {
        self.server.host == "localhost"
            || self.server.host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    }

    pub fn allow_private_network(&self) -> bool {
        self.upstream
            .allow_private_network
            .unwrap_or_else(|| self.listens_on_loopback())
    }

    pub fn is_trusted(&self, base: &str) -> bool {
        self.upstream.trusted.iter().any(|t| t == base)
    }

    pub fn forwards_request_header(&self, name: &HeaderName) -> bool {
        Self::passes(name, &self.headers.request_allow, &self.headers.request_deny)
    }

    pub fn forwards_response_header(&self, name: &HeaderName) -> bool {
        Self::passes(name, &self.headers.response_allow, &self.headers.response_deny)
    }

    fn passes(name: &HeaderName, allow: &[String], deny: &[String]) -> bool {
        let n = name.as_str();
        if !allow.is_empty() && !allow.iter().any(|p| Self::matches(n, p)) {
            return false;
        }
        !deny.iter().any(|p| Self::matches(n, p))
    }

    fn matches(name: &str, pattern: &str) -> bool {
        match pattern.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix),
            None => name == pattern,
        }
    }

    pub fn verify_ttl(&self) -> Duration {
        Duration::from_secs(self.upstream.verify_ttl_secs)
    }
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.connect_timeout_secs)
    }
    pub fn body_read_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.body_read_timeout_secs)
    }
    pub fn upstream_headers_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.upstream_headers_timeout_secs)
    }
    pub fn stream_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.stream_idle_timeout_secs)
    }
    pub fn verify_timeout(&self) -> Duration {
        Duration::from_secs(self.limits.verify_timeout_secs)
    }
}
