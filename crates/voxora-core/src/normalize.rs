//! Stage 1: deterministic cleanup. Fast (< 1 ms), never invents content, always available as the
//! fallback when the LLM is off, slow, or fails the invariant guard.

use std::sync::LazyLock;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::math;
use crate::mode::Mode;

/// User vocabulary: exact spellings to enforce, plus phrase replacements / snippets.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Dictionary {
    /// Canonical spellings ("PyTorch", "FAISS", "LangGraph"). Case-insensitive matches are
    /// rewritten to this exact casing, and the words bias Whisper via its initial prompt.
    pub terms: Vec<String>,
    /// Spoken phrase → replacement text ("pie torch" → "PyTorch", "my email" → "me@x.com").
    pub replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

/// Compiled matcher for a [`Dictionary`]; build once per settings change.
pub struct DictMatcher {
    ac: Option<AhoCorasick>,
    targets: Vec<String>,
}

impl DictMatcher {
    pub fn new(dict: &Dictionary) -> Self {
        let mut patterns = Vec::new();
        let mut targets = Vec::new();
        for r in &dict.replacements {
            if !r.from.trim().is_empty() {
                patterns.push(r.from.trim().to_string());
                targets.push(r.to.clone());
            }
        }
        for t in dict.terms.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            patterns.push(t.to_string());
            targets.push(t.to_string());
            // ASR often splits compound names: "lang graph" → LangGraph, "fast api" → FastAPI.
            let spaced = split_compound(t);
            if spaced.contains(' ') {
                patterns.push(spaced);
                targets.push(t.to_string());
            }
        }
        let ac = (!patterns.is_empty()).then(|| {
            AhoCorasickBuilder::new()
                .ascii_case_insensitive(true)
                .match_kind(MatchKind::LeftmostLongest)
                .build(&patterns)
                .expect("dictionary patterns are plain literals")
        });
        Self { ac, targets }
    }

    /// Replace whole-word, case-insensitive matches.
    pub fn apply(&self, text: &str) -> String {
        let Some(ac) = &self.ac else { return text.to_string() };
        let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for m in ac.find_iter(text) {
            let before = text[..m.start()].chars().next_back();
            let after = text[m.end()..].chars().next();
            if is_word(before) || is_word(after) {
                continue;
            }
            out.push_str(&text[last..m.start()]);
            out.push_str(&self.targets[m.pattern().as_usize()]);
            last = m.end();
        }
        out.push_str(&text[last..]);
        out
    }
}

/// "LangGraph" → "Lang Graph", "FastAPI" → "Fast API", "NumPy" → "Num Py", "BM25" → "BM25".
fn split_compound(term: &str) -> String {
    let chars: Vec<char> = term.chars().collect();
    let mut s = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1);
        let boundary = c.is_uppercase()
            && prev.is_some_and(|p| p.is_lowercase() || (p.is_uppercase() && next.is_some_and(|n| n.is_lowercase())));
        if boundary {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

#[derive(Debug, Clone, Copy)]
pub struct NormalizeOptions {
    pub mode: Mode,
    /// "new line" / "new paragraph" become line breaks.
    pub spoken_commands: bool,
    /// Add a final period to sentence-like text (off for terminals).
    pub terminal_period: bool,
}

static FILLERS: LazyLock<Regex> = LazyLock::new(|| {
    // Fillers with their trailing comma/space; "er"/"ah" only as whole words.
    Regex::new(r"(?i)\b(?:u+m+|u+h+m*|e+r+m+|e+r|a+h+|h+m+|m+h+m+)\b[,.]?\s*").unwrap()
});
static SPACE_BEFORE_PUNCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]+([,.;:!?])").unwrap());
static DOUBLE_COMMA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r",\s*([,.;:!?])").unwrap());
/// "64, no wait, 128" → "128": a spoken number immediately corrected by another number.
static NUMBER_CORRECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\d[\d.:,]*(?:\s*(?:%|percent|[ap]\.?m\.?))?,?\s+(?:no,?\s+)?(?:wait|weight|sorry|i mean|actually|make (?:it|that)|rather)[,.]?\s+(\d)")
        .unwrap()
});
static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]{2,}").unwrap());
static NEW_PARAGRAPH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[,.]?\s*\bnew paragraph\b[,.]?\s*").unwrap());
static NEW_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[,.]?\s*\bnew line\b[,.]?\s*").unwrap());
static CASE_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(snake|camel|pascal|kebab|constant|screaming snake) case ((?:[A-Za-z0-9]+[ ]?){1,6})").unwrap()
});

/// Words that end an identifier phrase: "camel case user id and save" → `userId and save`.
/// Deliberately excludes "by"/"of"/"for"/"in", which are common inside names
/// (`get_user_by_id`, `num_of_items`).
const IDENT_STOP: &[&str] = &[
    "the", "and", "or", "to", "with", "that", "is", "a", "an", "from", "as", "it", "then", "but", "so", "which",
    "into", "using", "should", "will", "please",
];

/// Whisper sometimes literalizes the command itself: "camelCaseGetUserById", "SnakeCaseMaxRetries".
static LITERAL_CASE_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?i:(snake|camel|pascal|kebab|constant)[ _-]?case)([A-Z0-9][A-Za-z0-9]*)\b").unwrap()
});

pub fn normalize(raw: &str, dict: &DictMatcher, opts: NormalizeOptions) -> String {
    let mut s = raw.trim().replace(['\r', '\n'], " ");
    s = dict.apply(&s);
    if opts.mode == Mode::Verbatim {
        return tidy(&s);
    }
    s = FILLERS.replace_all(&s, "").into_owned();
    s = collapse_stutters(&s);
    s = NUMBER_CORRECTION.replace_all(&s, "$1").into_owned();
    if opts.spoken_commands {
        s = NEW_PARAGRAPH.replace_all(&s, ".\n\n").into_owned();
        s = NEW_LINE.replace_all(&s, "\n").into_owned();
    }
    if opts.mode == Mode::Technical || opts.mode == Mode::Math {
        s = identifier_commands(&s);
    }
    s = tidy(&s);
    if let Some(style) = opts.mode.math_style() {
        s = math::convert(&s, style);
    }
    s = capitalize_starts(&s);
    if opts.terminal_period && !math::is_whole_math(&s) && word_count(&s) >= 3 {
        let trimmed = s.trim_end();
        if trimmed.ends_with(|c: char| c.is_alphanumeric() || "\u{E001})\"%'".contains(c)) {
            s = format!("{trimmed}.");
        }
    }
    match opts.mode.math_style() {
        Some(style) => math::finalize(&s, style),
        None => math::strip(&s),
    }
}

fn tidy(s: &str) -> String {
    let s = DOUBLE_COMMA.replace_all(s, "$1");
    let s = SPACE_BEFORE_PUNCT.replace_all(&s, "$1");
    let s = MULTI_SPACE.replace_all(&s, " ");
    let s = s.split('\n').map(|l| l.trim().trim_start_matches([',', ';']).trim_start()).collect::<Vec<_>>().join("\n");
    // "\n\n" from "new paragraph" after a sentence that already ended: drop the doubled period.
    s.replace("..\n", ".\n")
        .replace("?.\n", "?\n")
        .replace("!.\n", "!\n")
        .trim_start_matches(['.', '\n'])
        .trim()
        .to_string()
}

/// "the the" → "the", "I I think" → "I think". Keeps legitimate doubles like "that that".
fn collapse_stutters(s: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for w in s.split(' ') {
        let dup = out.last().is_some_and(|p| {
            let norm = |x: &str| x.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            !w.is_empty()
                && norm(p) == norm(w)
                && !norm(w).is_empty()
                && !["that", "had", "is", "do"].contains(&norm(w).as_str())
                && !p.ends_with(|c: char| ".,;:!?".contains(c))
        });
        if !dup {
            out.push(w);
        }
    }
    out.join(" ")
}

/// "snake case user id" → `user_id`, "camel case get user name" → `getUserName`.
fn identifier_commands(s: &str) -> String {
    let literal = LITERAL_CASE_CMD.replace_all(s, |c: &regex::Captures| {
        let words: Vec<String> = split_compound(&c[2]).split(' ').map(str::to_lowercase).collect();
        format_ident(&c[1].to_lowercase(), &words)
    });
    let s = literal.as_ref();
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for c in CASE_CMD.captures_iter(s) {
        let all = c.get(0).unwrap();
        let style = c[1].to_lowercase();
        let mut words: Vec<String> = Vec::new();
        let mut consumed_end = c.get(2).unwrap().start();
        for m in regex_words(&c[2], c.get(2).unwrap().start()) {
            if IDENT_STOP.contains(&m.0.to_lowercase().as_str()) {
                break;
            }
            words.push(m.0.to_lowercase());
            consumed_end = m.1;
        }
        if words.is_empty() {
            continue;
        }
        out.push_str(&s[last..all.start()]);
        out.push_str(&format_ident(&style, &words));
        last = consumed_end;
    }
    out.push_str(&s[last..]);
    out
}

fn regex_words(s: &str, offset: usize) -> Vec<(String, usize)> {
    let mut v = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
        if c.is_ascii_alphanumeric() {
            start.get_or_insert(i);
        } else if let Some(st) = start.take() {
            v.push((s[st..i].to_string(), offset + i));
        }
    }
    v
}

fn format_ident(style: &str, words: &[String]) -> String {
    let cap = |w: &str| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
    };
    match style {
        "snake" => words.join("_"),
        "kebab" => words.join("-"),
        "constant" | "screaming snake" => words.join("_").to_uppercase(),
        "pascal" => words.iter().map(|w| cap(w)).collect(),
        _ => {
            let mut s = words[0].clone();
            s.extend(words[1..].iter().map(|w| cap(w)));
            s
        }
    }
}

/// Uppercase the first letter of the text and of each line (not after every period: Whisper
/// already sentence-cases, and "e.g. this" must stay lowercase).
fn capitalize_starts(s: &str) -> String {
    s.split('\n')
        .map(|line| {
            let mut c = line.chars();
            match c.next() {
                Some(f) if f.is_lowercase() => f.to_uppercase().chain(c).collect(),
                _ => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn word_count(s: &str) -> usize {
    s.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict() -> DictMatcher {
        DictMatcher::new(&Dictionary {
            terms: vec!["PyTorch".into(), "FAISS".into(), "BM25".into(), "LangGraph".into()],
            replacements: vec![Replacement { from: "pie torch".into(), to: "PyTorch".into() }],
        })
    }

    fn run(mode: Mode, s: &str) -> String {
        normalize(s, &dict(), NormalizeOptions { mode, spoken_commands: true, terminal_period: true })
    }

    #[test]
    fn removes_fillers_and_fixes_spacing() {
        assert_eq!(run(Mode::Clean, "um, so we should, uh, ship it today"), "So we should, ship it today.");
        assert_eq!(run(Mode::Clean, "Hmm let me think about it"), "Let me think about it.");
    }

    #[test]
    fn keeps_real_words_that_look_like_fillers() {
        let s = "The error is in the umbrella module, ah no, the hummus parser.";
        assert_eq!(run(Mode::Clean, s), "The error is in the umbrella module, no, the hummus parser.");
    }

    #[test]
    fn dictionary_enforces_casing_and_whole_words() {
        assert_eq!(run(Mode::Clean, "use faiss and bm25 with pie torch"), "Use FAISS and BM25 with PyTorch.");
        assert_eq!(run(Mode::Clean, "build it with lang graph"), "Build it with LangGraph.");
        assert_eq!(split_compound("FastAPI"), "Fast API");
        assert_eq!(split_compound("BM25"), "BM25");
        // "faissy" is not the term.
        assert_eq!(run(Mode::Verbatim, "faissy"), "faissy");
    }

    #[test]
    fn verbatim_only_applies_dictionary() {
        assert_eq!(run(Mode::Verbatim, "um so  pie torch"), "um so PyTorch");
    }

    #[test]
    fn stutters_collapse_but_that_that_survives() {
        assert_eq!(run(Mode::Clean, "the the model said that that works"), "The model said that that works.");
    }

    #[test]
    fn numeric_self_corrections() {
        assert_eq!(run(Mode::Clean, "The batch size should be 64, no weight, 128."), "The batch size should be 128.");
        assert_eq!(run(Mode::Clean, "set it to 10%, actually 20%"), "Set it to 20%.");
        assert_eq!(run(Mode::Clean, "meet at 3 pm, sorry, 4 pm"), "Meet at 4 pm.");
        assert_eq!(run(Mode::Verbatim, "64, no wait, 128"), "64, no wait, 128");
        assert_eq!(
            run(Mode::Clean, "we had 3 options, actually the best one won"),
            "We had 3 options, actually the best one won."
        );
    }

    #[test]
    fn spoken_line_breaks() {
        assert_eq!(run(Mode::Clean, "first point new line second point"), "First point\nSecond point.");
        assert_eq!(
            run(Mode::Clean, "That is the intro. New paragraph. Now the details"),
            "That is the intro.\n\nNow the details."
        );
    }

    #[test]
    fn identifier_commands_in_technical_mode() {
        assert_eq!(
            run(Mode::Technical, "rename it to camel case user id count and save"),
            "Rename it to userIdCount and save."
        );
        assert_eq!(run(Mode::Technical, "call snake case load model weights"), "Call load_model_weights");
        assert_eq!(run(Mode::Technical, "set constant case max retries"), "Set MAX_RETRIES");
        assert_eq!(
            run(Mode::Technical, "rename it to camel case get user by id and update callers"),
            "Rename it to getUserById and update callers."
        );
        // Whisper literalized the command into one token.
        assert_eq!(run(Mode::Technical, "Rename it to camelCaseGetUserById now"), "Rename it to getUserById now.");
        assert_eq!(run(Mode::Technical, "Add SnakeCaseMaxRetries here"), "Add max_retries here.");
        // Not applied outside technical modes.
        assert_eq!(run(Mode::Clean, "snake case is popular"), "Snake case is popular.");
    }

    #[test]
    fn short_commands_get_no_period_and_math_gets_no_capital() {
        assert_eq!(run(Mode::Clean, "git status"), "Git status");
        assert_eq!(run(Mode::Technical, "x squared plus y squared"), "x² + y²");
        assert_eq!(run(Mode::Math, "sigma from i equals 1 to n of x sub i"), r"\sum_{i=1}^{n} x_i");
    }

    #[test]
    fn terminal_period_can_be_disabled() {
        let out = normalize(
            "list all the running containers",
            &dict(),
            NormalizeOptions { mode: Mode::Technical, spoken_commands: true, terminal_period: false },
        );
        assert_eq!(out, "List all the running containers");
    }
}
