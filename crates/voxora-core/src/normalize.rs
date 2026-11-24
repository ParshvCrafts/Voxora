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

/// "scratch that" (optionally "whole sentence" / "last part") drops the sentence it follows.
static SCRATCH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[,;]?\s*\bscratch that(?: whole sentence| last sentence| sentence| last part)?\b[,.;:]?\s*")
        .unwrap()
});
/// "quote, Our R² reached 0.841, end quote," → `"Our R² reached 0.841,"` (punctuation inside).
static QUOTE_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:open )?quote[,:]?\s+(.+?)([,.!?]?)\s+(?:end quote|unquote|close quote)\b([,.!?]?)").unwrap()
});
/// A spoken final "period" ("...until the period ends, period."). The comma keeps "grace period." safe.
static TRAILING_PERIOD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i),\s*period[.!]?\s*$").unwrap());
/// Discourse openers that carry no content: "So basically, I was thinking" → "I was thinking".
static OPENER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(^|[.!?]\s+)(?:so,?\s+basically|basically|so),\s*|(^|[.!?]\s+)so basically\s+").unwrap()
});
static R_SQUARED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[Rr][- ]squared\b").unwrap());
/// "p less than 0.001" → "p < 0.001". Single-letter variables only, never "a" or "I".
static COMPARISON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b([b-zB-HJ-Z]) (less than or equal to|greater than or equal to|less than|greater than) (-?\d[\d.,]*\d|\d)",
    )
    .unwrap()
});
static FLAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(?:dash dash|double dash) ([A-Za-z])").unwrap());
static FLAG_VALUE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(--[A-Za-z][\w-]*) equals (\S+)").unwrap());
static BIG_O: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:[Bb]ig )?[Oo] of ").unwrap());
static ORDINAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(^|[:;,.]\s*)(?:and\s+)?(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth)(?:ly)?,\s+",
    )
    .unwrap()
});

pub fn normalize(raw: &str, dict: &DictMatcher, opts: NormalizeOptions) -> String {
    let mut s = raw.trim().replace(['\r', '\n'], " ");
    s = dict.apply(&s);
    if opts.mode == Mode::Verbatim {
        return tidy(&s);
    }
    s = FILLERS.replace_all(&s, "").into_owned();
    if opts.mode == Mode::Technical {
        // Before stutter collapsing, which would turn "dash dash" into "dash".
        s = FLAG.replace_all(&s, "--$1").into_owned();
    }
    s = collapse_stutters(&s);
    s = NUMBER_CORRECTION.replace_all(&s, "$1").into_owned();
    if opts.spoken_commands {
        s = scratch_that(&s);
        s = QUOTE_CMD
            .replace_all(&s, |c: &regex::Captures| {
                let p = if c[2].is_empty() { &c[3] } else { &c[2] };
                format!("\"{}{p}\"", &c[1])
            })
            .into_owned();
        s = TRAILING_PERIOD.replace(&s, ".").into_owned();
        s = NEW_PARAGRAPH.replace_all(&s, ".\n\n").into_owned();
        s = NEW_LINE.replace_all(&s, "\n").into_owned();
    }
    s = OPENER.replace_all(&s, "$1$2").into_owned();
    s = R_SQUARED.replace_all(&s, "R²").into_owned();
    s = COMPARISON
        .replace_all(&s, |c: &regex::Captures| {
            let op = match &c[2] {
                "less than" => "<",
                "greater than" => ">",
                "less than or equal to" => "≤",
                _ => "≥",
            };
            format!("{} {op} {}", &c[1], &c[3])
        })
        .into_owned();
    if opts.mode == Mode::Technical || opts.mode == Mode::Math {
        s = identifier_commands(&s);
    }
    if opts.mode == Mode::Technical {
        s = FLAG_VALUE.replace_all(&s, "$1=$2").into_owned();
        s = spoken_symbols(&s);
        s = big_o(&s);
    }
    s = repair_paths(&s);
    s = tidy(&s);
    if opts.mode != Mode::Math {
        s = ordinal_list(&s);
    }
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

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Drop everything from the start of the current sentence up to and including the cue.
fn scratch_that(s: &str) -> String {
    let mut s = s.to_string();
    while let Some(m) = SCRATCH.find(&s) {
        let before = &s[..m.start()];
        let keep = before.rfind(['.', '!', '?', '\n']).map_or(0, |i| i + 1);
        let after = upper_first(&s[m.end()..]);
        s = format!("{} {after}", &before[..keep]).trim().to_string();
    }
    s
}

/// File extensions and TLDs that make "name dot ext" a filename/host rather than prose.
const DOT_SUFFIXES: &[&str] = &[
    "com", "org", "net", "io", "ai", "dev", "app", "edu", "gov", "co", "uk", "py", "rs", "js", "ts", "tsx", "jsx",
    "json", "md", "txt", "csv", "yaml", "yml", "toml", "html", "css", "sh", "exe", "dll", "cpp", "h", "c", "go",
    "java", "ipynb", "pdf", "png", "jpg", "env", "lock", "log", "sql", "db", "xml", "zip", "ps1", "bat", "cfg", "ini",
    "wav", "mp4",
];

/// Technical mode: spoken URL/path/identifier punctuation between tokens.
/// "test underscore user dot py" → `test_user.py`,
/// "https colon slash slash api dot example dot com slash v1" → `https://api.example.com/v1`,
/// "C colon backslash temp backslash out dot json" → `C:\temp\out.json`.
/// A span needs a strong joiner (slash, backslash, underscore, or "dot" before a known
/// extension/TLD or between digits), so "alpha equals 0.001" and "the red dot" stay prose.
fn spoken_symbols(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    let symbol = |i: usize| -> Option<(&'static str, usize)> {
        let w = |k: usize| words.get(k).map(|x| x.to_lowercase());
        match w(i)?.as_str() {
            "dot" => Some((".", 1)),
            "slash" => Some(("/", 1)),
            "backslash" => Some(("\\", 1)),
            "colon" => Some((":", 1)),
            "underscore" => Some(("_", 1)),
            "equals" => Some(("=", 1)),
            "ampersand" => Some(("&", 1)),
            "question" if w(i + 1).as_deref() == Some("mark") => Some(("?", 2)),
            "back" | "forward" if w(i + 1).as_deref() == Some("slash") => {
                Some((if w(i) == Some("back".into()) { "\\" } else { "/" }, 2))
            }
            _ => None,
        }
    };
    let is_word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_alphanumeric() || "-.".contains(c));
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let first = words[i];
        if symbol(i).is_some() || !is_word(first) {
            out.push(first.to_string());
            i += 1;
            continue;
        }
        // Greedily extend: word (symbol+ word)*. The last word may carry trailing punctuation.
        let mut parts: Vec<(Vec<&str>, String)> = vec![(vec![], first.to_string())];
        let mut j = i + 1;
        let mut tail = String::new();
        loop {
            let mut syms = vec![];
            let mut k = j;
            while let Some((sym, n)) = symbol(k) {
                syms.push(sym);
                k += n;
            }
            let Some(next) = words.get(k).filter(|_| !syms.is_empty()) else { break };
            let core = next.trim_end_matches(|c: char| ",.;:!?".contains(c));
            if !is_word(core) || symbol(k).is_some() {
                break;
            }
            parts.push((syms, core.to_string()));
            j = k + 1;
            if core.len() != next.len() {
                tail = next[core.len()..].to_string();
                break;
            }
        }
        let strong = parts.iter().enumerate().skip(1).any(|(n, (syms, w))| {
            syms.iter().any(|s| ["/", "\\", "_"].contains(s))
                || (syms == &["."]
                    && (DOT_SUFFIXES.contains(&w.to_lowercase().as_str())
                        || (w.chars().all(|c| c.is_ascii_digit())
                            && parts[n - 1].1.chars().all(|c| c.is_ascii_digit()))))
        });
        if parts.len() < 2 || !strong {
            out.push(first.to_string());
            i += 1;
            continue;
        }
        let url = parts.iter().any(|(syms, _)| syms.concat() == ":/" || syms.concat() == "://");
        let mut joined = String::new();
        let mut in_host = url;
        let last = parts.len() - 1;
        for (n, (syms, w)) in parts.iter().enumerate() {
            let sym = syms.concat();
            if url && n > 0 && sym == "/" {
                in_host = false;
            }
            joined.push_str(&sym);
            let ext = n == last && sym == "." && DOT_SUFFIXES.contains(&w.to_lowercase().as_str());
            joined.push_str(&if in_host || ext { w.to_lowercase() } else { w.clone() });
        }
        out.push(joined + &tail);
        i = j;
    }
    out.join(" ")
}

static URL_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(https?|ftp):[\\/]+(\S+)").unwrap());
static DRIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([a-z]):\\").unwrap());
static UPPER_EXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([\w-]+)\.([A-Z][A-Za-z0-9]{0,4})\b").unwrap());

/// Fix what Whisper does to written paths: `https:\api.example.com/v1\users` (mixed slashes),
/// `c:\temp` (lowercase drive), `out.JSON` (the dictionary uppercased an extension).
fn repair_paths(s: &str) -> String {
    let s = URL_START.replace_all(s, |c: &regex::Captures| {
        let rest = c[2].replace('\\', "/");
        let (host, path) = rest.split_once('/').map_or((rest.as_str(), None), |(h, p)| (h, Some(p)));
        let path = path.map(|p| format!("/{p}")).unwrap_or_default();
        format!("{}://{}{path}", c[1].to_lowercase(), host.to_lowercase())
    });
    let s = DRIVE.replace_all(&s, |c: &regex::Captures| format!("{}:\\", c[1].to_uppercase()));
    UPPER_EXT
        .replace_all(&s, |c: &regex::Captures| {
            let ext = c[2].to_lowercase();
            if DOT_SUFFIXES.contains(&ext.as_str()) { format!("{}.{ext}", &c[1]) } else { c[0].to_string() }
        })
        .into_owned()
}

/// "O of V plus E log V" → `O(V + E log V)`, protected from the math parser.
fn big_o(s: &str) -> String {
    const STOP: &[&str] = &[
        "using", "with", "while", "when", "in", "for", "and", "but", "which", "time", "space", "because", "on", "if",
        "is", "was", "where", "so", "that", "the", "a", "an",
    ];
    let mut out = String::new();
    let mut rest = s;
    while let Some(m) = BIG_O.find(rest) {
        out.push_str(&rest[..m.start()]);
        let after = &rest[m.end()..];
        let mut terms: Vec<String> = Vec::new();
        let mut consumed = 0;
        for w in after.split(' ') {
            let core = w.trim_end_matches(|c: char| ",.;:!?".contains(c));
            let low = core.to_lowercase();
            if core.is_empty() || STOP.contains(&low.as_str()) || !core.chars().all(char::is_alphanumeric) {
                break;
            }
            match low.as_str() {
                "plus" => terms.push("+".into()),
                "minus" => terms.push("-".into()),
                "times" => terms.push("·".into()),
                "squared" => terms.last_mut().map_or((), |t| t.push('²')),
                "cubed" => terms.last_mut().map_or((), |t| t.push('³')),
                _ => terms.push(core.to_string()),
            }
            consumed += w.len() + 1;
            if core.len() != w.len() {
                break; // trailing punctuation ends the expression
            }
        }
        if terms.is_empty() {
            out.push_str(m.as_str());
            rest = after;
            continue;
        }
        let consumed = consumed.min(after.len());
        let used = &after[..consumed];
        let punct_tail = used.trim_end().len() - used.trim_end().trim_end_matches(|c: char| ",.;:!?".contains(c)).len();
        out.push(math::OPEN);
        out.push_str(&format!("O({})", terms.join(" ")));
        out.push(math::CLOSE);
        let used_trim = used.trim_end();
        out.push_str(&used_trim[used_trim.len() - punct_tail..]);
        if used.ends_with(' ') {
            out.push(' ');
        }
        rest = &after[consumed..];
    }
    out.push_str(rest);
    out
}

/// "Here are three blockers: first, A; second, B; and third, C." → a numbered list.
/// Needs "first" then consecutive ordinals (at least two items), each at a clause start.
fn ordinal_list(s: &str) -> String {
    const ORD: &[&str] =
        &["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth"];
    let caps: Vec<_> = ORDINAL.captures_iter(s).collect();
    let Some(start) = caps.iter().position(|c| c[2].eq_ignore_ascii_case("first")) else { return s.to_string() };
    let mut run = vec![&caps[start]];
    for c in &caps[start + 1..] {
        if !ORD.get(run.len()).is_some_and(|want| c[2].eq_ignore_ascii_case(want)) {
            break;
        }
        run.push(c);
    }
    if run.len() < 2 {
        return s.to_string();
    }
    let first = run[0];
    let mut intro = s[..first.get(1).unwrap().end()].trim_end().trim_end_matches([',', ';']).to_string();
    if intro.ends_with(|c: char| c.is_alphanumeric()) {
        intro.push(':');
    }
    let last = run[run.len() - 1].get(0).unwrap().end();
    let end = s[last..].find(". ").map_or(s.len(), |i| last + i + 1);
    let mut items = Vec::new();
    for (n, c) in run.iter().enumerate() {
        let from = c.get(0).unwrap().end();
        let to = run.get(n + 1).map_or(end, |next| next.get(0).unwrap().start());
        let item = s[from..to].trim().trim_end_matches([',', ';', '.']).trim();
        items.push(format!("{}. {}.", n + 1, upper_first(item)));
    }
    let rest = s[end..].trim();
    let mut out = if intro.is_empty() { items.join("\n") } else { format!("{intro}\n{}", items.join("\n")) };
    if !rest.is_empty() {
        out = format!("{out}\n{rest}");
    }
    out
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
                // "slash slash", "underscore underscore init": spoken symbols repeat on purpose.
                && !["that", "had", "is", "do", "slash", "backslash", "dash", "underscore", "dot"]
                    .contains(&norm(w).as_str())
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
    fn golden_spoken_urls_paths_and_flags() {
        let t = |s| run(Mode::Technical, s);
        assert_eq!(
            t(
                "Run pytest dash dash maxfail equals 1 on test underscore user underscore auth dot py and check camel case get user by ID."
            ),
            "Run pytest --maxfail=1 on test_user_auth.py and check getUserById."
        );
        assert_eq!(
            t(
                "Fetch https colon slash slash API dot example dot com slash v1 slash users question mark limit equals 50 and save to C colon backslash temp backslash out dot JSON."
            ),
            "Fetch https://api.example.com/v1/users?limit=50 and save to C:\\temp\\out.json."
        );
        assert_eq!(t("Benchmark Q4 underscore K underscore M today"), "Benchmark Q4_K_M today.");
        // Prose stays prose: no strong joiner.
        assert_eq!(t("click the red dot then save"), "Click the red dot then save.");
        assert_eq!(t("version 3 dot 5 ships"), "Version 3.5 ships.");
        // Clean mode never joins.
        assert_eq!(run(Mode::Clean, "open test underscore file"), "Open test underscore file.");
    }

    #[test]
    fn whisper_path_artifacts_are_repaired() {
        // Real whisper-server output for golden case 10.
        assert_eq!(
            run(Mode::Technical, "fetch https:\\api.example.com/v1\\users?limit=50 and save to c:\\temp\\out.json"),
            "Fetch https://api.example.com/v1/users?limit=50 and save to C:\\temp\\out.json."
        );
        assert_eq!(run(Mode::Clean, "see Node.JS docs and README.MD"), "See Node.js docs and README.md.");
    }

    #[test]
    fn golden_big_o_is_formatted_and_protected_from_math() {
        assert_eq!(
            run(
                Mode::Technical,
                "The worst-case time complexity is O of V plus E log V using a binary min-heap, while space stays O of V."
            ),
            "The worst-case time complexity is O(V + E log V) using a binary min-heap, while space stays O(V)."
        );
        assert_eq!(run(Mode::Technical, "sorting is big O of n log n"), "Sorting is O(n log n).");
    }

    #[test]
    fn golden_quotes_trailing_period_and_comparisons() {
        assert_eq!(
            run(
                Mode::Clean,
                "Reply to Dr. Patel saying, quote, Our R-squared reached 0.841 with p less than 0.001, end quote, and attach the CSV."
            ),
            "Reply to Dr. Patel saying, \"Our R² reached 0.841 with p < 0.001,\" and attach the CSV."
        );
        assert_eq!(
            run(
                Mode::Clean,
                "Send a Slack message saying we need a full stop on new features until the period ends, period."
            ),
            "Send a Slack message saying we need a full stop on new features until the period ends."
        );
        // A real trailing "period" (no comma) is a word, and "a less than" is not a variable.
        assert_eq!(run(Mode::Clean, "we are in the grace period."), "We are in the grace period.");
        assert_eq!(run(Mode::Clean, "a less than 5 percent chance"), "A less than 5 percent chance.");
    }

    #[test]
    fn golden_scratch_that_and_openers() {
        assert_eq!(
            run(
                Mode::Clean,
                "Tell the team the migration is complete, scratch that whole sentence, tell them the migration is paused pending QA sign-off."
            ),
            "Tell them the migration is paused pending QA sign-off."
        );
        assert_eq!(
            run(Mode::Clean, "We ship Monday. Use the old build, scratch that, use the new one."),
            "We ship Monday. Use the new one."
        );
        assert_eq!(run(Mode::Clean, "Um, so basically I was thinking we refactor"), "I was thinking we refactor.");
    }

    #[test]
    fn golden_ordinal_list() {
        assert_eq!(
            run(
                Mode::Clean,
                "Here are three blockers: first, CUDA out-of-memory on batch size 64; second, missing foreign key indexes; and third, flaky WebSocket reconnects."
            ),
            "Here are three blockers:\n1. CUDA out-of-memory on batch size 64.\n2. Missing foreign key indexes.\n3. Flaky WebSocket reconnects."
        );
        // One ordinal is not a list; "at first," is not a clause-start ordinal.
        assert_eq!(run(Mode::Clean, "First, we eat."), "First, we eat.");
        assert_eq!(run(Mode::Clean, "at first, it failed; second, it hung"), "At first, it failed; second, it hung.");
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
