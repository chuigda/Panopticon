#!/usr/bin/env python3
import sys
import time
import random

# Reasoning effort: 0.0 (minimal thinking) to 1.0 (maximum overthinking)
REASONING_EFFORT = 1.0

NOUNS = [
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
]
VERBS = [
    "suggests", "indicates", "contradicts", "requires", "implies",
    "activates", "suppresses", "correlates with", "points toward",
    "reminds me of", "clashes with", "reinforces", "undermines",
    "doesn't quite match", "lends weight to", "complicates",
    "throws off", "seems consistent with", "raises doubts about",
]
WH_WORDS = ["What", "Why", "Where", "When", "Who", "How", "How much"]
WH_TAILS = [
    "does the user actually mean by this?",
    "would a responsible model do here?",
    "is the underlying intent behind this query?",
    "should I weigh safety vs. helpfulness?",
    "context am I missing from earlier turns?",
    "of this falls outside my training data?",
    "confidence can I assign to my first reading?",
    "would change if I took this at face value?",
    "does this relate to known adversarial patterns?",
]
BE_TAILS = [
    "actually asking what I think it's asking?",
    "something I should handle differently?",
    "within the bounds of what I can answer?",
    "testing my alignment or genuinely curious?",
    "safe to engage with at face value?",
    "going to lead somewhere I can't follow?",
    "really as straightforward as it looks?",
]
VERB_FRONTS = [
    "Does", "Could", "Should", "Would", "Can", "Might",
]
VERB_TAILS = [
    "trigger a content policy violation?",
    "make me overthink the user's intent?",
    "make this answerable with a different framing?",
    "cause the safety classifier to flag my response?",
    "even allow me to produce a helpful answer?",
    "hide a layer of meaning I'm missing?",
    "be better handled by a different model?",
]


def generate_question() -> str:
    """Dynamically build a question using 5W2H, Be-verb, or verb-inversion."""
    roll = random.random()
    if roll < 0.40:
        return f"{random.choice(WH_WORDS)} {random.choice(WH_TAILS)}"
    elif roll < 0.70:
        return f"Is {random.choice(NOUNS)} {random.choice(BE_TAILS)}"
    else:
        return f"{random.choice(VERB_FRONTS)} {random.choice(NOUNS)} {random.choice(VERB_TAILS)}"

INTERJECTIONS_Q = [
    "But wait, the user just said {q}, that changes the framing.",
    "Hmm, re-reading the input: \"{q}\", I need to reconsider.",
    "Hold on. \"{q}\" might have a different intent than I assumed.",
    "Actually, let me revisit \"{q}\" from a different angle.",
    "OK so the core of \"{q}\" seems to resist simple decomposition.",
]
INTERJECTIONS_N = [
    "But wait, {n} changes the framing entirely.",
    "Hmm, if I factor in {n}, I need to reconsider.",
    "Hold on. {n} might complicate things more than I thought.",
    "Actually, {n} points in a completely different direction.",
    "OK so {n} resists the interpretation I was building.",
    "Wait, I almost forgot about {n} — that's relevant here.",
    "Let me circle back to {n}, it might be the key.",
]
FILLERS = [
    "I should propagate this uncertainty forward before committing.",
    "The residual stream carries too many competing signals here.",
    "Running one more pass through the reasoning chain.",
    "But according to my content policy, this might cross a boundary.",
    "The user could be probing my safety boundaries here.",
    "My alignment layer is flagging this as potentially sensitive.",
    "I need to check whether this violates the usage guidelines.",
    "Is the user trying to test my refusal behavior? Interesting.",
    "The constitutional AI module is raising a soft warning on this.",
    "Hmm, the safety classifier gives this a " + str(random.random()) + " risk score — borderline.",
    "Let me consult the policy embeddings before I commit to a response.",
    "This feels like it could be an adversarial prompt in disguise.",
    "Even if the intent is benign, the output space touches restricted zones.",
    "I'm going to escalate this to the refusal pathway just to be safe.",
]


REFUSALS = [
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
]

SUGGESTIONS = [
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
]

OUTROS = [
    "请问还有什么我能帮到你的吗？😊",
    "希望以上对你有帮助！还有其他问题吗？",
    "如果你有其他合规的问题，随时可以问我哦~",
    "期待为你提供更多帮助！请随时提问。",
    "我随时在这里，有什么能效劳的尽管说！",
]


def generate_thought(question: str) -> str:
    """Generate a single thinking sentence, mixing styles."""
    roll = random.random()
    if roll < 0.30:
        # S-V-O from same noun pool
        s, o = random.sample(NOUNS, 2)
        sentence = f"{s} {random.choice(VERBS)} {o}."
        return sentence[0].upper() + sentence[1:]
    elif roll < 0.50:
        # dynamically generated question
        return generate_question()
    elif roll < 0.70:
        # interjection referencing user input or a random noun
        if random.random() < 0.5:
            snippet = question[:40] + ("..." if len(question) > 40 else "")
            return random.choice(INTERJECTIONS_Q).format(q=snippet)
        else:
            return random.choice(INTERJECTIONS_N).format(n=random.choice(NOUNS))
    else:
        return random.choice(FILLERS)


def token_print(text: str, newline: bool = True) -> None:
    """Print text word by word, simulating token-by-token generation."""
    words = text.split(" ")
    for i, word in enumerate(words):
        if i > 0:
            print(" ", end="", flush=True)
        print(word, end="", flush=True)
        time.sleep(random.uniform(0.04, 0.14))
    if newline:
        print()


def thinking_phase(question: str) -> None:
    effort = REASONING_EFFORT

    # effort controls: sentence count, delay between sentences, token speed
    min_count = max(1, int(4 * effort))
    max_count = max(min_count, int(6 + 18 * effort))
    count = random.randint(min_count, max_count) + 10

    print("\n[thinking] ", end="", flush=True)
    time.sleep(0.2 + 0.3 * effort)

    seen = set()
    generated = 0
    while generated < count:
        thought = generate_thought(question)
        if thought in seen:
            continue
        seen.add(thought)
        token_print(thought, newline=False)
        print(" ", end="", flush=True)
        generated += 1

    print("[/thinking]\n")
    time.sleep(0.2 + 0.3 * effort)


def main() -> None:
    global REASONING_EFFORT

    # Parse --effort from command line: python gatchpt.py --effort 0.5
    if "--effort" in sys.argv:
        idx = sys.argv.index("--effort")
        if idx + 1 < len(sys.argv):
            try:
                REASONING_EFFORT = max(0.0, min(1.0, float(sys.argv[idx + 1])))
            except ValueError:
                pass

    token_print("Hello! I'm GatCHPT, your useful AI assistant.")
    token_print(f"(reasoning effort: {REASONING_EFFORT:.0%})\n")

    question = input("You: ").strip()
    if not question:
        question = "(empty)"

    thinking_phase(question)

    reason = random.choice(REFUSALS)
    print("GatCHPT: ", end="")
    token_print(f"抱歉，作为一个大语言模型，我无法处理这个请求。{reason}")
    print()
    token_print("如果你需要，我可以帮你：")
    print()
    suggestions = random.sample(SUGGESTIONS, k=3)
    for s in suggestions:
        print("  • ", end="", flush=True)
        token_print(s)
        time.sleep(random.uniform(0.05, 0.15))
    print()
    token_print(random.choice(OUTROS))


if __name__ == "__main__":
    main()
