//! Text pipeline: raw ASR text → final text, with the LLM as an optional, guarded polish step.
//!
//! ```text
//! raw ──► Stage 1 normalize ──► fast path? ──yes──► done
//!                                   │ no
//!                                   ▼
//!                     Stage 3 LLM (timeout) ──► Stage 4 guard ──ok──► done
//!                                   │ error/timeout         │ violation
//!                                   └──────────► Stage 1 text ◄┘
//! ```

use std::future::Future;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use regex::Regex;
use serde_json::Value;

use crate::context::Profile;
use crate::guard;
use crate::mode::Mode;
use crate::normalize::{DictMatcher, NormalizeOptions, normalize, word_count};
use crate::prompts;

/// Anything that can turn chat messages into a reply (llama-server, Ollama, a test double).
pub trait Rewriter {
    fn complete(&self, messages: Vec<Value>, max_tokens: u32) -> impl Future<Output = Result<String, String>> + Send;
}

pub struct Request<'a> {
    pub raw: &'a str,
    pub profile: Profile,
    pub app_exe: &'a str,
    pub dict: &'a DictMatcher,
    pub terms: &'a [String],
    pub spoken_commands: bool,
    /// Always skip the LLM at or below this many words.
    pub fast_path_words: usize,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Path {
    /// Verbatim mode or LLM disabled.
    Deterministic,
    /// Short or already-clean utterance: LLM skipped for speed.
    FastPath,
    Llm,
    /// LLM tried and rejected; reason recorded for diagnostics.
    Fallback(String),
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub text: String,
    pub deterministic: String,
    pub path: Path,
    pub llm_ms: Option<u64>,
}

static DISFLUENCY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:u+m+|u+h+|e+r+m+|like,|you know|i mean|actually|sorry|scratch that|no wait|wait no|kind of|sort of|basically|so yeah)\b")
        .unwrap()
});

/// Would the LLM add anything? Clean-ish modes skip it for short, fluent speech.
pub fn needs_llm(raw: &str, mode: Mode, fast_path_words: usize) -> bool {
    if !mode.uses_llm() {
        return false;
    }
    let words = word_count(raw);
    if words <= fast_path_words {
        return false;
    }
    let disfluent = DISFLUENCY.is_match(raw) || has_repeat(raw) || guard::has_correction(raw);
    match mode {
        Mode::Clean | Mode::Technical | Mode::Math => disfluent || words > 25,
        _ => true,
    }
}

/// `regex` has no backreferences, so detect "the the" here.
fn has_repeat(raw: &str) -> bool {
    let w: Vec<String> =
        raw.split_whitespace().map(|x| x.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).collect();
    w.windows(2).any(|p| {
        !p[0].is_empty()
            && p[0] == p[1]
            && !["that", "had", "slash", "backslash", "dash", "underscore", "dot"].contains(&p[0].as_str())
    })
}

pub async fn process<R: Rewriter>(req: Request<'_>, llm: Option<&R>) -> Outcome {
    let mode = req.profile.mode;
    let det = normalize(
        req.raw,
        req.dict,
        NormalizeOptions { mode, spoken_commands: req.spoken_commands, terminal_period: req.profile.terminal_period },
    );
    let done = |path| Outcome { text: det.clone(), deterministic: det.clone(), path, llm_ms: None };
    let Some(llm) = llm.filter(|_| mode.uses_llm()) else { return done(Path::Deterministic) };
    if det.is_empty() || !needs_llm(req.raw, mode, req.fast_path_words) {
        return done(Path::FastPath);
    }

    let messages = prompts::messages(mode, &det, req.app_exe, req.terms);
    // Rewrites are about as long as the input; JSON wrapper + slack on top.
    let max_tokens = (det.len() / 3 + 48).min(2048) as u32;
    let started = Instant::now();
    let reply = tokio::time::timeout(req.timeout, llm.complete(messages, max_tokens)).await;
    let llm_ms = Some(started.elapsed().as_millis() as u64);
    let fallback =
        |why: String| Outcome { text: det.clone(), deterministic: det.clone(), path: Path::Fallback(why), llm_ms };

    let reply = match reply {
        Err(_) => return fallback(format!("timeout after {} ms", req.timeout.as_millis())),
        Ok(Err(e)) => return fallback(format!("llm error: {e}")),
        Ok(Ok(r)) => r,
    };
    let Some(mut cand) = prompts::parse_reply(&reply) else {
        return fallback("unparseable reply".into());
    };
    cand = req.dict.apply(&cand);
    if !req.profile.terminal_period && !det.ends_with('.') && cand.ends_with('.') && !cand.ends_with("..") {
        cand.pop();
    }
    if let Err(v) = guard::check(req.raw, &det, &cand, mode) {
        return fallback(format!("guard: {v}"));
    }
    Outcome { text: cand, deterministic: det, path: Path::Llm, llm_ms }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::PasteKeys;
    use crate::normalize::Dictionary;

    struct Fake(Result<&'static str, &'static str>, u64);
    impl Rewriter for Fake {
        async fn complete(&self, _: Vec<Value>, _: u32) -> Result<String, String> {
            tokio::time::sleep(Duration::from_millis(self.1)).await;
            self.0.map(String::from).map_err(String::from)
        }
    }

    fn req<'a>(raw: &'a str, mode: Mode, dict: &'a DictMatcher) -> Request<'a> {
        Request {
            raw,
            profile: Profile { mode, paste: PasteKeys::CtrlV, terminal_period: true },
            app_exe: "notepad.exe",
            dict,
            terms: &[],
            spoken_commands: true,
            fast_path_words: 3,
            timeout: Duration::from_millis(200),
        }
    }

    const MESSY: &str = "um so we should uh move the sync to Tuesday, actually no, Wednesday";

    #[tokio::test]
    async fn blueprint_short_fluent_command_skips_llm() {
        let d = DictMatcher::new(&Dictionary::default());
        let llm = Fake(Ok(r#"{"text":"WRONG"}"#), 0);
        let o = process(req("Run pytest on the auth module.", Mode::Technical, &d), Some(&llm)).await;
        assert_eq!((o.path, o.text.as_str()), (Path::FastPath, "Run pytest on the auth module."));
    }

    #[tokio::test]
    async fn uses_llm_when_it_passes_the_guard() {
        let d = DictMatcher::new(&Dictionary::default());
        let llm = Fake(Ok(r#"{"text":"We should move the sync to Wednesday."}"#), 0);
        let o = process(req(MESSY, Mode::Clean, &d), Some(&llm)).await;
        assert_eq!((o.path, o.text.as_str()), (Path::Llm, "We should move the sync to Wednesday."));
    }

    #[tokio::test]
    async fn blueprint_llm_timeout_falls_back_to_deterministic() {
        let d = DictMatcher::new(&Dictionary::default());
        let llm = Fake(Ok(r#"{"text":"late"}"#), 2_000);
        let o = process(req(MESSY, Mode::Clean, &d), Some(&llm)).await;
        assert!(matches!(o.path, Path::Fallback(ref w) if w.starts_with("timeout")), "{:?}", o.path);
        assert_eq!(o.text, o.deterministic);
        assert!(o.text.starts_with("So we should move the sync"), "{}", o.text);
    }

    #[tokio::test]
    async fn guard_violation_and_errors_fall_back() {
        let d = DictMatcher::new(&Dictionary::default());
        let hallucinated = Fake(Ok(r#"{"text":"We should move the sync to Wednesday at 9 AM."}"#), 0);
        let o = process(req(MESSY, Mode::Clean, &d), Some(&hallucinated)).await;
        assert!(matches!(o.path, Path::Fallback(ref w) if w.contains("guard")), "{:?}", o.path);

        let down = Fake(Err("connection refused"), 0);
        let o = process(req(MESSY, Mode::Clean, &d), Some(&down)).await;
        assert!(matches!(o.path, Path::Fallback(ref w) if w.contains("connection refused")));

        let garbage = Fake(Ok("Sure! Here you go"), 0);
        assert!(matches!(process(req(MESSY, Mode::Clean, &d), Some(&garbage)).await.path, Path::Fallback(_)));
    }

    #[tokio::test]
    async fn verbatim_and_no_llm_are_deterministic() {
        let d = DictMatcher::new(&Dictionary::default());
        let llm = Fake(Ok(r#"{"text":"x"}"#), 0);
        assert_eq!(process(req(MESSY, Mode::Verbatim, &d), Some(&llm)).await.path, Path::Deterministic);
        assert_eq!(process::<Fake>(req(MESSY, Mode::Clean, &d), None).await.path, Path::Deterministic);
    }

    #[test]
    fn fast_path_rules() {
        assert!(!needs_llm("git status", Mode::Professional, 3));
        assert!(needs_llm("send the report to the team today", Mode::Professional, 3));
        assert!(!needs_llm("send the report to the team today", Mode::Clean, 3));
        assert!(needs_llm("send the the report today", Mode::Clean, 3));
        assert!(needs_llm("um send the report today", Mode::Clean, 3));
        // ASR heard "no wait" as "no weight": still a self-correction.
        assert!(needs_llm("The batch size should be 64, no weight, 128.", Mode::Clean, 3));
    }
}
