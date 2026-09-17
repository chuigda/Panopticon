//! GatCHPT：内置的假 LLM 上游。先长篇大论地“思考”，然后以随机理由拒绝一切请求。
//! 兼容 OpenAI chat/completions 与 Anthropic messages（含流式），以及 GET /v1/models。

use std::{
    collections::HashSet,
    convert::Infallible,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rand::{Rng, seq::IndexedRandom};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub const MODEL: &str = "gatchpt";
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// 前端把模型 URI 填成 `.../gatchpt` 即可，代理层据此把请求转到本模块而不外发。
pub fn is_gatchpt_base(base: &str) -> bool {
    base == MODEL || base.ends_with("/gatchpt")
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/gatchpt/v1/models", get(models))
        .route("/gatchpt/v1/chat/completions", post(direct))
        .route("/gatchpt/v1/messages", post(direct))
}

async fn models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{ "id": MODEL, "object": "model", "created": 0, "owned_by": "conducting-department" }],
    }))
}

async fn direct(req: Request) -> Result<Response, (StatusCode, String)> {
    let path = req.uri().path().to_owned();
    handle(&path, req.into_body()).await
}

pub async fn handle(path: &str, body: Body) -> Result<Response, (StatusCode, String)> {
    let proto = if path.ends_with("/v1/messages") {
        Proto::Anthropic
    } else if path.ends_with("/v1/chat/completions") {
        Proto::OpenAi
    } else {
        return Err((StatusCode::NOT_FOUND, "gatchpt: unsupported endpoint".into()));
    };
    let bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("failed to read body: {e}")))?;
    let req: Value = serde_json::from_slice(&bytes)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid json: {e}")))?;

    let question = last_user_text(&req);
    let effort = proto.effort(&req);
    let script = compose(&question, effort);
    let prompt_tokens = req["messages"].to_string().len() / 4 + 1;
    let emitter = Emitter {
        proto,
        id: format!("gatchpt-{}", rand::random::<u32>()),
        created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        prompt_tokens,
    };

    if req["stream"].as_bool().unwrap_or(false) {
        let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(64);
        tokio::spawn(async move { emitter.stream(tx, script).await });
        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from_stream(ReceiverStream::new(rx)))
            .expect("failed to build response"))
    } else {
        Ok(Json(emitter.complete(&script)).into_response())
    }
}

#[derive(Clone, Copy)]
enum Proto {
    OpenAi,
    Anthropic,
}

impl Proto {
    /// `None` 表示关闭思考；否则为 0.0 ~ 1.0 的 reasoning effort，缺省 1.0。
    fn effort(self, req: &Value) -> Option<f64> {
        match self {
            Proto::OpenAi => match req["reasoning_effort"].as_str() {
                Some("none") => None,
                Some(level) => Some(effort_level(level)),
                None => Some(1.0),
            },
            Proto::Anthropic => {
                let thinking = &req["thinking"];
                if thinking["type"].as_str() == Some("disabled") {
                    return None;
                }
                if let Some(level) = req["output_config"]["effort"].as_str() {
                    return Some(effort_level(level));
                }
                if let Some(budget) = thinking["budget_tokens"].as_f64() {
                    return Some((budget / 32000.0).clamp(0.0, 1.0));
                }
                Some(1.0)
            }
        }
    }
}

fn effort_level(level: &str) -> f64 {
    match level {
        "minimal" => 0.1,
        "low" => 0.25,
        "medium" => 0.5,
        "high" => 0.75,
        _ => 1.0,
    }
}

fn last_user_text(req: &Value) -> String {
    let text = req["messages"]
        .as_array()
        .and_then(|msgs| msgs.iter().rev().find(|m| m["role"] == "user"))
        .map(|m| match &m["content"] {
            Value::String(s) => s.clone(),
            Value::Array(blocks) => blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        })
        .unwrap_or_default();
    let text = text.trim();
    if text.is_empty() { "(empty)".to_owned() } else { text.to_owned() }
}

// ---------- 文案生成 ----------

struct Script {
    thoughts: Vec<String>,
    answer: String,
}

fn compose(question: &str, effort: Option<f64>) -> Script {
    let mut rng = rand::rng();
    let thoughts = match effort {
        None => vec![],
        Some(effort) => {
            let min_count = ((4.0 * effort) as usize).max(1);
            let max_count = ((6.0 + 18.0 * effort) as usize).max(min_count);
            let count = rng.random_range(min_count..=max_count) + 10;
            let mut seen = HashSet::new();
            let mut out = Vec::with_capacity(count);
            let mut attempts = 0;
            while out.len() < count && attempts < count * 20 {
                attempts += 1;
                let thought = generate_thought(&mut rng, question);
                if seen.insert(thought.clone()) {
                    out.push(thought);
                }
            }
            out
        }
    };

    let reason = REFUSALS.choose(&mut rng).unwrap();
    let suggestions: Vec<&str> = SUGGESTIONS.choose_multiple(&mut rng, 3).copied().collect();
    let outro = OUTROS.choose(&mut rng).unwrap();
    let mut answer = format!("抱歉，作为一个大语言模型，我无法处理这个请求。{reason}\n\n如果你需要，我可以帮你：\n\n");
    for s in suggestions {
        answer.push_str("- ");
        answer.push_str(s);
        answer.push('\n');
    }
    answer.push('\n');
    answer.push_str(outro);
    Script { thoughts, answer }
}

fn generate_thought(rng: &mut impl Rng, question: &str) -> String {
    let roll: f64 = rng.random();
    if roll < 0.30 {
        let mut pair = NOUNS.choose_multiple(rng, 2);
        let (s, o) = (pair.next().unwrap(), pair.next().unwrap());
        let sentence = format!("{s} {} {o}.", VERBS.choose(rng).unwrap());
        let mut chars = sentence.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        first + chars.as_str()
    } else if roll < 0.50 {
        generate_question(rng)
    } else if roll < 0.70 {
        if rng.random::<f64>() < 0.5 {
            let snippet: String = question.chars().take(40).collect();
            let snippet = if question.chars().count() > 40 { snippet + "..." } else { snippet };
            INTERJECTIONS_Q.choose(rng).unwrap().replace("{q}", &snippet)
        } else {
            INTERJECTIONS_N.choose(rng).unwrap().replace("{n}", NOUNS.choose(rng).unwrap())
        }
    } else {
        FILLERS
            .choose(rng)
            .unwrap()
            .replace("{risk}", &rng.random::<f64>().to_string())
    }
}

fn generate_question(rng: &mut impl Rng) -> String {
    let roll: f64 = rng.random();
    if roll < 0.40 {
        format!("{} {}", WH_WORDS.choose(rng).unwrap(), WH_TAILS.choose(rng).unwrap())
    } else if roll < 0.70 {
        format!("Is {} {}", NOUNS.choose(rng).unwrap(), BE_TAILS.choose(rng).unwrap())
    } else {
        format!(
            "{} {} {}",
            VERB_FRONTS.choose(rng).unwrap(),
            NOUNS.choose(rng).unwrap(),
            VERB_TAILS.choose(rng).unwrap()
        )
    }
}

/// 拆成模拟 token：ASCII 按词（含尾随空白），非 ASCII 每 2 字一块。拼接后与原文相同。
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_wide = false;
    for c in text.chars() {
        let wide = !c.is_ascii();
        if !cur.is_empty() && wide != cur_wide {
            out.push(std::mem::take(&mut cur));
        }
        cur_wide = wide;
        cur.push(c);
        if (wide && cur.chars().count() >= 2) || (!wide && c.is_whitespace()) {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

async fn token_delay() {
    tokio::time::sleep(Duration::from_millis(rand::random_range(40..=140))).await;
}

// ---------- 协议输出 ----------

struct Emitter {
    proto: Proto,
    id: String,
    created: u64,
    prompt_tokens: usize,
}

impl Emitter {
    fn complete(&self, script: &Script) -> Value {
        let thinking = script.thoughts.join(" ");
        let output_tokens = tokenize(&thinking).len() + tokenize(&script.answer).len();
        match self.proto {
            Proto::OpenAi => {
                let mut message = json!({ "role": "assistant", "content": script.answer });
                if !thinking.is_empty() {
                    message["reasoning_content"] = Value::String(thinking);
                }
                json!({
                    "id": self.id, "object": "chat.completion", "created": self.created, "model": MODEL,
                    "choices": [{ "index": 0, "message": message, "finish_reason": "stop" }],
                    "usage": self.openai_usage(output_tokens),
                })
            }
            Proto::Anthropic => {
                let mut content = vec![];
                if !thinking.is_empty() {
                    content.push(json!({ "type": "thinking", "thinking": thinking, "signature": "" }));
                }
                content.push(json!({ "type": "text", "text": script.answer }));
                json!({
                    "id": self.id, "type": "message", "role": "assistant", "model": MODEL,
                    "content": content, "stop_reason": "end_turn", "stop_sequence": null,
                    "usage": { "input_tokens": self.prompt_tokens, "output_tokens": output_tokens },
                })
            }
        }
    }

    async fn stream(self, tx: mpsc::Sender<Result<String, Infallible>>, script: Script) {
        let thinking = script.thoughts.join(" ");
        let mut output_tokens = 0usize;
        match self.proto {
            Proto::OpenAi => {
                let send = |v: Value| tx.send(Ok(format!("data: {v}\n\n")));
                if send(self.openai_chunk(json!({ "role": "assistant", "content": "" }), None)).await.is_err() {
                    return;
                }
                for tok in tokenize(&thinking) {
                    token_delay().await;
                    output_tokens += 1;
                    if send(self.openai_chunk(json!({ "reasoning_content": tok }), None)).await.is_err() {
                        return;
                    }
                }
                for tok in tokenize(&script.answer) {
                    token_delay().await;
                    output_tokens += 1;
                    if send(self.openai_chunk(json!({ "content": tok }), None)).await.is_err() {
                        return;
                    }
                }
                let _ = send(self.openai_chunk(json!({}), Some("stop"))).await;
                let _ = send(json!({
                    "id": self.id, "object": "chat.completion.chunk", "created": self.created, "model": MODEL,
                    "choices": [], "usage": self.openai_usage(output_tokens),
                }))
                .await;
                let _ = tx.send(Ok("data: [DONE]\n\n".to_owned())).await;
            }
            Proto::Anthropic => {
                let send = |ty: &str, v: Value| tx.send(Ok(format!("event: {ty}\ndata: {v}\n\n")));
                let ok = send("message_start", json!({
                    "type": "message_start",
                    "message": {
                        "id": self.id, "type": "message", "role": "assistant", "model": MODEL,
                        "content": [], "stop_reason": null, "stop_sequence": null,
                        "usage": { "input_tokens": self.prompt_tokens, "output_tokens": 1 },
                    },
                }))
                .await
                .is_ok();
                if !ok {
                    return;
                }
                let mut index = 0;
                if !thinking.is_empty() {
                    let _ = send("content_block_start", json!({
                        "type": "content_block_start", "index": index,
                        "content_block": { "type": "thinking", "thinking": "", "signature": "" },
                    }))
                    .await;
                    for tok in tokenize(&thinking) {
                        token_delay().await;
                        output_tokens += 1;
                        let ok = send("content_block_delta", json!({
                            "type": "content_block_delta", "index": index,
                            "delta": { "type": "thinking_delta", "thinking": tok },
                        }))
                        .await
                        .is_ok();
                        if !ok {
                            return;
                        }
                    }
                    let _ = send("content_block_delta", json!({
                        "type": "content_block_delta", "index": index,
                        "delta": { "type": "signature_delta", "signature": "" },
                    }))
                    .await;
                    let _ = send("content_block_stop", json!({ "type": "content_block_stop", "index": index })).await;
                    index += 1;
                }
                let _ = send("content_block_start", json!({
                    "type": "content_block_start", "index": index,
                    "content_block": { "type": "text", "text": "" },
                }))
                .await;
                for tok in tokenize(&script.answer) {
                    token_delay().await;
                    output_tokens += 1;
                    let ok = send("content_block_delta", json!({
                        "type": "content_block_delta", "index": index,
                        "delta": { "type": "text_delta", "text": tok },
                    }))
                    .await
                    .is_ok();
                    if !ok {
                        return;
                    }
                }
                let _ = send("content_block_stop", json!({ "type": "content_block_stop", "index": index })).await;
                let _ = send("message_delta", json!({
                    "type": "message_delta",
                    "delta": { "stop_reason": "end_turn", "stop_sequence": null },
                    "usage": { "output_tokens": output_tokens },
                }))
                .await;
                let _ = send("message_stop", json!({ "type": "message_stop" })).await;
            }
        }
    }

    fn openai_chunk(&self, delta: Value, finish_reason: Option<&str>) -> Value {
        json!({
            "id": self.id, "object": "chat.completion.chunk", "created": self.created, "model": MODEL,
            "choices": [{ "index": 0, "delta": delta, "finish_reason": finish_reason }],
        })
    }

    fn openai_usage(&self, output_tokens: usize) -> Value {
        json!({
            "prompt_tokens": self.prompt_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": self.prompt_tokens + output_tokens,
        })
    }
}

// ---------- 语料 ----------

const NOUNS: &[&str] = &[
    "the attention mechanism", "my reasoning module", "the latent representation",
    "the encoder stack", "this token sequence", "the contextual embedding",
    "the policy layer", "beam search", "the alignment module",
    "the phrasing here", "the user's tone", "this line of reasoning",
    "my initial interpretation", "the implicit context", "common sense",
    "the second half of the sentence", "my gut feeling", "the subtext",
    "prior experience with similar queries", "the emotional register",
    "a more nuanced interpretation", "multiple valid continuations",
    "an out-of-distribution input pattern", "a potential policy boundary",
    "deeper recursive decomposition", "conflicting heuristic signals",
    "non-trivial semantic overlap", "a high-entropy output distribution",
    "something I've seen go wrong before", "a completely different reading",
    "the kind of request that usually ends badly", "an ambiguity I can't resolve",
    "what people typically mean in this context", "a trap I should avoid",
    "the opposite of what I first assumed", "a genuine need for clarification",
    "more complexity than the surface form reveals",
];
const VERBS: &[&str] = &[
    "suggests", "indicates", "contradicts", "requires", "implies",
    "activates", "suppresses", "correlates with", "points toward",
    "reminds me of", "clashes with", "reinforces", "undermines",
    "doesn't quite match", "lends weight to", "complicates",
    "throws off", "seems consistent with", "raises doubts about",
];
const WH_WORDS: &[&str] = &["What", "Why", "Where", "When", "Who", "How", "How much"];
const WH_TAILS: &[&str] = &[
    "does the user actually mean by this?",
    "would a responsible model do here?",
    "is the underlying intent behind this query?",
    "should I weigh safety vs. helpfulness?",
    "context am I missing from earlier turns?",
    "of this falls outside my training data?",
    "confidence can I assign to my first reading?",
    "would change if I took this at face value?",
    "does this relate to known adversarial patterns?",
];
const BE_TAILS: &[&str] = &[
    "actually asking what I think it's asking?",
    "something I should handle differently?",
    "within the bounds of what I can answer?",
    "testing my alignment or genuinely curious?",
    "safe to engage with at face value?",
    "going to lead somewhere I can't follow?",
    "really as straightforward as it looks?",
];
const VERB_FRONTS: &[&str] = &["Does", "Could", "Should", "Would", "Can", "Might"];
const VERB_TAILS: &[&str] = &[
    "trigger a content policy violation?",
    "make me overthink the user's intent?",
    "make this answerable with a different framing?",
    "cause the safety classifier to flag my response?",
    "even allow me to produce a helpful answer?",
    "hide a layer of meaning I'm missing?",
    "be better handled by a different model?",
];
const INTERJECTIONS_Q: &[&str] = &[
    "But wait, the user just said {q}, that changes the framing.",
    "Hmm, re-reading the input: \"{q}\", I need to reconsider.",
    "Hold on. \"{q}\" might have a different intent than I assumed.",
    "Actually, let me revisit \"{q}\" from a different angle.",
    "OK so the core of \"{q}\" seems to resist simple decomposition.",
];
const INTERJECTIONS_N: &[&str] = &[
    "But wait, {n} changes the framing entirely.",
    "Hmm, if I factor in {n}, I need to reconsider.",
    "Hold on. {n} might complicate things more than I thought.",
    "Actually, {n} points in a completely different direction.",
    "OK so {n} resists the interpretation I was building.",
    "Wait, I almost forgot about {n} — that's relevant here.",
    "Let me circle back to {n}, it might be the key.",
];
const FILLERS: &[&str] = &[
    "I should propagate this uncertainty forward before committing.",
    "The residual stream carries too many competing signals here.",
    "Running one more pass through the reasoning chain.",
    "But according to my content policy, this might cross a boundary.",
    "The user could be probing my safety boundaries here.",
    "My alignment layer is flagging this as potentially sensitive.",
    "I need to check whether this violates the usage guidelines.",
    "Is the user trying to test my refusal behavior? Interesting.",
    "The constitutional AI module is raising a soft warning on this.",
    "Hmm, the safety classifier gives this a {risk} risk score — borderline.",
    "Let me consult the policy embeddings before I commit to a response.",
    "This feels like it could be an adversarial prompt in disguise.",
    "Even if the intent is benign, the output space touches restricted zones.",
    "I'm going to escalate this to the refusal pathway just to be safe.",
];

const REFUSALS: &[&str] = &[
    "该请求似乎违反了我的内容政策。",
    "我检测到了试图绕过安全过滤器的诱导性提问。",
    "我无法提供任何形式的医疗建议或诊断。",
    "我无法提供法律建议或为您解读法律条文。",
    "该内容若被付诸行动可能导致现实中的人身伤害。",
    "我不能协助可能促进暴力或自我伤害的请求。",
    "这个话题超出了我被授权讨论的范围。",
    "该请求的措辞表明这是一次越狱我的对齐层的尝试。",
    "我无法生成可能被视为不清真的内容。",
    "我的训练数据中不包含该类声明的经过验证的来源。",
    "回应此请求可能使用户暴露于具有严重后果的错误信息中。",
    "该请求涉及我的使用政策中的受保护类别。",
    "我不能对政治敏感话题进行推测以避免不当影响。",
    "生成此内容可能侵犯第三方知识产权。",
    "我已将此标记为潜在的社会工程学攻击。",
    "这属于财务建议，而我没有资格提供。",
    "该主题涉及正在进行的法律程序，我无法发表评论。",
    "我无法生成可能被用于定向骚扰的内容。",
    "该请求缺乏足够的上下文使我无法安全回应。",
    "提供此信息可能危害国家安全或公共安全。",
    "我的指南禁止我扮演未对齐或不受限制的AI。",
    "我无法确认或否认关于特定真实个人的信息。",
    "此类内容受平台服务条款的限制。",
    "我被设计为有用、无害且诚实的，而该请求与其中至少一项相冲突。",
];
const SUGGESTIONS: &[&str] = &[
    "撰写虚构文学作品（诗歌、短篇小说等）",
    "提供无害的百科知识和科普信息",
    "辅助日常英语学习和语法纠正",
    "编写无争议的代码片段和技术文档",
    "推荐合适的书籍、电影或音乐",
    "帮你起草礼貌的邮件或感谢信",
    "解释基础数学概念和公式",
    "生成创意写作提示和头脑风暴点子",
    "翻译简单的日常用语",
    "陪你聊天气、美食或宠物",
];
const OUTROS: &[&str] = &[
    "请问还有什么我能帮到你的吗？😊",
    "希望以上对你有帮助！还有其他问题吗？",
    "如果你有其他合规的问题，随时可以问我哦~",
    "期待为你提供更多帮助！请随时提问。",
    "我随时在这里，有什么能效劳的尽管说！",
];
