//! Reproducible quality + latency benchmark (blueprint §10).
//!
//! ```text
//! cargo run --release --bin bench_eval -- [--audio bench/audio] [--no-llm] [--runs 5]
//!     [--llm http://127.0.0.1:18080/v1] [--whisper-port 18181] [--out bench/report.md]
//! ```
//! Without `--audio`, the spoken text is fed straight to the rewrite pipeline (isolates the
//! text stages). With it, each case's WAV goes through whisper-server first and WER is scored.
//! Servers must already be running (the app starts them; see docs/BENCHMARKS.md).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;
use voxora_core::asr_filter;
use voxora_core::context::{PasteKeys, Profile};
use voxora_core::engines::llm::OpenAiCompat;
use voxora_core::engines::whisper::{WhisperClient, decode_wav, vocabulary_prompt};
use voxora_core::mode::Mode;
use voxora_core::normalize::DictMatcher;
use voxora_core::pipeline::{self, Path, Request};
use voxora_core::settings::Settings;

#[derive(Deserialize)]
struct Case {
    id: String,
    category: String,
    mode: Mode,
    spoken: String,
    #[serde(default)]
    expect: Vec<String>,
    #[serde(default)]
    expect_exact: Option<String>,
    #[serde(default)]
    forbid: Vec<String>,
    /// Extra personal-dictionary terms for this case (simulates a user who added them).
    #[serde(default)]
    terms: Vec<String>,
    /// The ideal output. Scored separately from the key-fact checks (whitespace-insensitive).
    #[serde(default)]
    target: Option<String>,
    /// WAV to use instead of `<id>.wav` (e.g. a noise-only clip).
    #[serde(default)]
    audio: Option<String>,
    /// Nothing may be inserted (noise / silence cases).
    #[serde(default)]
    expect_empty: bool,
}

struct Row {
    id: String,
    category: String,
    raw: String,
    out: String,
    path: Path,
    wer: Option<f64>,
    pass: bool,
    exact: Option<bool>,
    why: String,
    asr_ms: Vec<u64>,
    text_ms: Vec<u64>,
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned())
}
fn flag(name: &str) -> bool {
    std::env::args().any(|x| x == name)
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| c.is_whitespace() || ",.!?;:\"".contains(c))
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word error rate: word-level Levenshtein distance / reference length.
fn wer(reference: &str, hyp: &str) -> f64 {
    let (r, h) = (words(reference), words(hyp));
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    for i in 1..=r.len() {
        let mut cur = vec![i; h.len() + 1];
        for j in 1..=h.len() {
            let sub = prev[j - 1] + usize::from(r[i - 1] != h[j - 1]);
            cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
        }
        prev = cur;
    }
    prev[h.len()] as f64 / r.len() as f64
}

fn pct(v: &[u64], p: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    s[((s.len() as f64 - 1.0) * p).round() as usize]
}

/// Whitespace-insensitive comparison against the ideal output.
fn exact(target: &str, out: &str) -> bool {
    let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    squash(target) == squash(out)
}

/// The reference for WER: the spoken text without stage directions like `[pause 3500]`.
fn spoken_words(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '[' => depth += 1,
            ']' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn judge(c: &Case, out: &str) -> (bool, String) {
    if c.expect_empty {
        return if out.trim().is_empty() { (true, String::new()) } else { (false, "expected nothing".into()) };
    }
    if let Some(e) = &c.expect_exact
        && out != e
    {
        return (false, format!("expected exactly {e:?}"));
    }
    // Expectations with capitals (FAISS, getUserById) are case-sensitive; plain words are not.
    let has =
        |e: &str| if e.chars().any(char::is_uppercase) { out.contains(e) } else { out.to_lowercase().contains(e) };
    if let Some(m) = c.expect.iter().find(|e| !has(e)) {
        return (false, format!("missing {m:?}"));
    }
    let lo = out.to_lowercase();
    if let Some(f) = c.forbid.iter().find(|f| {
        let f = f.to_lowercase();
        // whole-word match so "um" doesn't trip on "number"
        lo.split(|ch: char| !ch.is_alphanumeric() && ch != ' ')
            .any(|seg| seg.split(' ').collect::<Vec<_>>().windows(f.split(' ').count()).any(|w| w.join(" ") == f))
    }) {
        return (false, format!("contains forbidden {f:?}"));
    }
    (true, String::new())
}

#[tokio::main]
async fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases_path = arg("--cases").map(PathBuf::from).unwrap_or(root.join("bench/cases.json"));
    let cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(&cases_path).expect("read cases")).expect("parse cases");
    let runs: usize = arg("--runs").and_then(|r| r.parse().ok()).unwrap_or(3);
    let audio = arg("--audio").map(PathBuf::from);
    let settings = Settings::default();
    let llm_url = arg("--llm").unwrap_or(settings.llm.endpoint.clone());
    let llm = (!flag("--no-llm")).then(|| OpenAiCompat::new(&llm_url, "voxora"));
    let whisper = WhisperClient::new(arg("--whisper-port").and_then(|p| p.parse().ok()).unwrap_or(settings.asr.port));

    if let Some(l) = &llm {
        l.warm_up().await.expect("LLM server not reachable; start llama-server first");
    }

    let mut rows = Vec::new();
    for c in &cases {
        if c.spoken.is_empty() && audio.is_none() {
            continue; // audio-only case (noise) in a text-only run
        }
        let mut d = settings.dictionary.clone();
        d.terms.extend(c.terms.iter().cloned());
        let dict = DictMatcher::new(&d);
        let terms = d.terms.clone();
        let prompt = vocabulary_prompt(&terms);
        let mut row = Row {
            id: c.id.clone(),
            category: if c.terms.is_empty() { c.category.clone() } else { format!("{} +dict", c.category) },
            raw: c.spoken.clone(),
            out: String::new(),
            path: Path::Deterministic,
            wer: None,
            pass: false,
            exact: None,
            why: String::new(),
            asr_ms: vec![],
            text_ms: vec![],
        };
        for run in 0..runs {
            let raw = if let Some(dir) = &audio {
                let file = c.audio.clone().unwrap_or(format!("{}.wav", c.id));
                let bytes = std::fs::read(dir.join(file)).expect("missing case WAV; run scripts/gen-bench-audio.ps1");
                let (samples, rate) = decode_wav(&bytes).expect("decode wav");
                assert_eq!(rate, 16_000, "bench WAVs must be 16 kHz");
                let t = Instant::now();
                let tr = whisper.transcribe(&samples, "en", &prompt).await.expect("whisper-server");
                row.asr_ms.push(t.elapsed().as_millis() as u64);
                asr_filter::clean(&tr.text, 10_000).unwrap_or_default()
            } else {
                spoken_words(&c.spoken).split_whitespace().collect::<Vec<_>>().join(" ")
            };
            let t = Instant::now();
            let o = pipeline::process(
                Request {
                    raw: &raw,
                    profile: Profile { mode: c.mode, paste: PasteKeys::CtrlV, terminal_period: true },
                    app_exe: "",
                    dict: &dict,
                    terms: &terms,
                    spoken_commands: true,
                    fast_path_words: settings.llm.fast_path_words,
                    timeout: Duration::from_millis(settings.llm.timeout_ms),
                },
                llm.as_ref(),
            )
            .await;
            row.text_ms.push(t.elapsed().as_millis() as u64);
            if run == 0 {
                row.wer = audio.as_ref().filter(|_| !c.spoken.is_empty()).map(|_| wer(&spoken_words(&c.spoken), &raw));
                (row.pass, row.why) = judge(c, &o.text);
                row.exact = c.target.as_deref().filter(|_| !c.expect_empty).map(|t| exact(t, &o.text));
                row.raw = raw;
                row.out = o.text;
                row.path = o.path;
            }
        }
        rows.push(row);
    }

    // Silence must produce nothing (whisper-server VAD + hallucination filter).
    let silence = match &audio {
        Some(dir) if dir.join("silence-5s.wav").exists() => {
            let (s, _) = decode_wav(&std::fs::read(dir.join("silence-5s.wav")).unwrap()).unwrap();
            let tr = whisper
                .transcribe(&s, "en", &vocabulary_prompt(&settings.dictionary.terms))
                .await
                .expect("whisper-server");
            Some(asr_filter::clean(&tr.text, 0))
        }
        _ => None,
    };

    let report = render(&rows, silence, runs, audio.is_some(), llm.is_some());
    println!("{report}");
    if let Some(out) = arg("--out") {
        std::fs::write(&out, &report).expect("write report");
        eprintln!("wrote {out}");
    }
}

fn render(rows: &[Row], silence: Option<Option<String>>, runs: usize, audio: bool, llm: bool) -> String {
    let passed = rows.iter().filter(|r| r.pass).count();
    let all_asr: Vec<u64> = rows.iter().flat_map(|r| r.asr_ms.clone()).collect();
    let llm_ms: Vec<u64> = rows.iter().filter(|r| r.path == Path::Llm).flat_map(|r| r.text_ms.clone()).collect();
    let fast_ms: Vec<u64> = rows.iter().filter(|r| r.path != Path::Llm).flat_map(|r| r.text_ms.clone()).collect();
    let totals: Vec<u64> = rows
        .iter()
        .flat_map(|r| r.text_ms.iter().enumerate().map(|(i, t)| t + r.asr_ms.get(i).copied().unwrap_or(0)))
        .collect();
    let wers: Vec<f64> = rows.iter().filter_map(|r| r.wer).collect();
    let fallbacks = rows.iter().filter(|r| matches!(r.path, Path::Fallback(_))).count();
    let llm_used = rows.iter().filter(|r| r.path == Path::Llm).count();

    let mut s = String::new();
    s += "## Summary\n\n";
    s += &format!(
        "- Cases: **{passed}/{} passed** key-fact checks ({runs} runs each; quality judged on run 1)\n",
        rows.len()
    );
    let scored: Vec<bool> = rows.iter().filter_map(|r| r.exact).collect();
    if !scored.is_empty() {
        s += &format!(
            "- Exact match with the ideal output: **{}/{}** (whitespace-insensitive)\n",
            scored.iter().filter(|e| **e).count(),
            scored.len()
        );
    }
    s += &format!(
        "- Input: {}; rewrite LLM: {}\n",
        if audio { "synthesized speech → whisper-server" } else { "reference text (ASR bypassed)" },
        if llm { "on" } else { "off" }
    );
    if !wers.is_empty() {
        s += &format!("- Mean ASR WER: **{:.1}%**\n", 100.0 * wers.iter().sum::<f64>() / wers.len() as f64);
    }
    if let Some(sil) = &silence {
        s += &format!(
            "- 5 s silence → {}\n",
            match sil {
                None => "**rejected (nothing inserted)**".to_string(),
                Some(t) => format!("**inserted {t:?}**"),
            }
        );
    }
    s += &format!(
        "- Paths: {llm_used} LLM, {fallbacks} guard/timeout fallbacks, {} fast/deterministic\n\n",
        rows.len() - llm_used - fallbacks
    );
    s += "| Stage | P50 | P95 | n |\n|---|---|---|---|\n";
    if !all_asr.is_empty() {
        s += &format!(
            "| ASR (whisper-server) | {} ms | {} ms | {} |\n",
            pct(&all_asr, 0.5),
            pct(&all_asr, 0.95),
            all_asr.len()
        );
    }
    if !llm_ms.is_empty() {
        s +=
            &format!("| Rewrite via LLM | {} ms | {} ms | {} |\n", pct(&llm_ms, 0.5), pct(&llm_ms, 0.95), llm_ms.len());
    }
    s += &format!(
        "| Rewrite fast path | {} ms | {} ms | {} |\n",
        pct(&fast_ms, 0.5),
        pct(&fast_ms, 0.95),
        fast_ms.len()
    );
    s += &format!("| ASR + rewrite | {} ms | {} ms | {} |\n\n", pct(&totals, 0.5), pct(&totals, 0.95), totals.len());

    s += "## Cases\n\n| | Exact | Case | Path | WER | Output | Note |\n|---|---|---|---|---|---|---|\n";
    for r in rows {
        let path = match &r.path {
            Path::Llm => "llm".to_string(),
            Path::FastPath => "fast".to_string(),
            Path::Deterministic => "det".to_string(),
            Path::Fallback(w) => format!("fallback: {}", w.chars().take(40).collect::<String>()),
        };
        let esc = |t: &str| t.replace('|', "\\|").replace('\n', "⏎");
        s += &format!(
            "| {} | {} | `{}` {} | {} | {} | {} | {} |\n",
            if r.pass { "✅" } else { "❌" },
            match r.exact {
                Some(true) => "=",
                Some(false) => "≠",
                None => "",
            },
            r.id,
            r.category,
            path,
            r.wer.map(|w| format!("{:.0}%", w * 100.0)).unwrap_or("-".into()),
            esc(&r.out),
            esc(&if r.raw != r.out && audio {
                format!("ASR: {}{}", r.raw, if r.why.is_empty() { String::new() } else { format!("; {}", r.why) })
            } else {
                r.why.clone()
            }),
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wer_counts_edits() {
        assert_eq!(wer("the cat sat", "the cat sat"), 0.0);
        assert!((wer("the cat sat", "the bat sat down") - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(wer("Hello, world!", "hello world"), 0.0);
    }
}
