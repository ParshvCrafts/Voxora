//! Stage 4: the post-LLM invariant guard. The LLM may polish wording; it may not change facts.
//!
//! The guard compares the deterministic text (what the user said, cleaned) against the LLM
//! candidate and rejects the candidate when it:
//! - introduces a number, code identifier or acronym that was never spoken,
//! - drops a spoken number or identifier (unless the speaker corrected themselves),
//! - answers or extends the dictation instead of rewriting it (new proper nouns, too much
//!   new vocabulary, chatty preamble),
//! - grows or shrinks implausibly.
//!
//! On rejection the caller inserts the deterministic text instead, so a misbehaving model costs
//! polish, never correctness.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::mode::Mode;
use crate::normalize::word_count;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Empty,
    Preamble,
    NewNumber(String),
    DroppedNumber(String),
    NewIdentifier(String),
    DroppedIdentifier(String),
    NewProperNoun(String),
    TooMuchNewContent { new: usize, total: usize },
    LengthRatio { before: usize, after: usize },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Empty => write!(f, "empty output"),
            Violation::Preamble => write!(f, "chatty preamble"),
            Violation::NewNumber(n) => write!(f, "introduced number {n:?}"),
            Violation::DroppedNumber(n) => write!(f, "dropped number {n:?}"),
            Violation::NewIdentifier(n) => write!(f, "introduced identifier {n:?}"),
            Violation::DroppedIdentifier(n) => write!(f, "dropped identifier {n:?}"),
            Violation::NewProperNoun(n) => write!(f, "introduced proper noun {n:?}"),
            Violation::TooMuchNewContent { new, total } => write!(f, "{new}/{total} content words are new"),
            Violation::LengthRatio { before, after } => write!(f, "length {before} -> {after} words"),
        }
    }
}

static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+(?:[.,:]\d+)*").unwrap());
static IDENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"`[^`]+`",                                   // backticked code
        r"|--?[a-z][a-z0-9-]*[a-z0-9]",               // CLI flags
        r"|\b[A-Za-z][A-Za-z0-9]*_[A-Za-z0-9_]+\b",   // snake_case / CONSTANT_CASE
        r"|\b[a-z]+[A-Z][A-Za-z0-9]*\b",              // camelCase
        r"|\b[A-Z][a-z0-9]+[A-Z][A-Za-z0-9]*\b",      // PascalCase with 2+ humps
        r"|\b[A-Za-z_]\w*\.[A-Za-z_]\w*(?:\.\w+)*\b", // dotted.names, file.py
        r"|\b[A-Z]{2,}[0-9]*\b",                      // ACRONYMS, BM25
        r"|\\[A-Za-z]+",                              // LaTeX commands
    ))
    .unwrap()
});
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z][A-Za-z'’-]*").unwrap());

/// Phrases that mean the speaker revised what they said, so facts may legitimately disappear.
pub const CORRECTION_CUES: &[&str] = &[
    "actually",
    "no wait",
    "no weight",
    "wait no",
    "i mean",
    "scratch that",
    "sorry",
    "rather",
    "correction",
    "make that",
    "make it",
    "change that",
    "no,",
    "instead",
];

const PREAMBLES: &[&str] = &[
    "here is",
    "here's",
    "sure",
    "certainly",
    "of course",
    "cleaned text",
    "rewritten",
    "the cleaned",
    "the rewritten",
    "output:",
    "i've ",
    "i have ",
];

const NUMBER_WORDS: &[(&str, &str)] = &[
    ("zero", "0"),
    ("one", "1"),
    ("two", "2"),
    ("three", "3"),
    ("four", "4"),
    ("five", "5"),
    ("six", "6"),
    ("seven", "7"),
    ("eight", "8"),
    ("nine", "9"),
    ("ten", "10"),
    ("eleven", "11"),
    ("twelve", "12"),
    ("fifteen", "15"),
    ("twenty", "20"),
    ("thirty", "30"),
    ("forty", "40"),
    ("fifty", "50"),
    ("hundred", "100"),
    ("thousand", "1000"),
    ("first", "1"),
    ("second", "2"),
    ("third", "3"),
    ("half", "2"),
];

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "to", "of", "in", "on", "at", "for", "with", "is", "are", "was", "were",
    "be", "been", "it", "its", "this", "that", "these", "those", "we", "you", "i", "they", "he", "she", "our", "your",
    "my", "me", "us", "them", "so", "as", "by", "from", "will", "would", "should", "could", "can", "do", "does", "did",
    "have", "has", "had", "not", "please", "let", "lets", "let's", "just", "also", "then", "there", "their", "what",
    "which", "who", "if", "into", "about", "more", "very", "all", "any", "some", "up", "out", "need", "want",
];

fn numbers(s: &str) -> Vec<String> {
    let lower = s.to_lowercase();
    let mut v: Vec<String> = NUMBER.find_iter(s).map(|m| m.as_str().replace(',', "")).collect();
    for w in WORD.find_iter(&lower) {
        if let Some((_, d)) = NUMBER_WORDS.iter().find(|(k, _)| *k == w.as_str()) {
            v.push((*d).to_string());
        }
    }
    v
}

fn identifiers(s: &str) -> Vec<String> {
    IDENT.find_iter(s).map(|m| m.as_str().trim_matches('`').to_string()).collect()
}

fn words_lower(s: &str) -> HashSet<String> {
    WORD.find_iter(s).map(|m| m.as_str().to_lowercase()).collect()
}

pub fn has_correction(raw: &str) -> bool {
    let l = raw.to_lowercase();
    CORRECTION_CUES.iter().any(|c| l.contains(c))
}

/// `source` = the deterministic (Stage 1) text; `raw` = untouched ASR text (for cue detection).
pub fn check(raw: &str, source: &str, candidate: &str, mode: Mode) -> Result<(), Violation> {
    let cand = candidate.trim();
    if cand.is_empty() {
        return Err(Violation::Empty);
    }
    let cl = cand.to_lowercase();
    let sl = source.to_lowercase();
    if PREAMBLES.iter().any(|p| cl.starts_with(p) && !sl.starts_with(p)) {
        return Err(Violation::Preamble);
    }

    let corrected = has_correction(raw);
    // Math mode legitimately turns words into symbols, so digits may appear from number words
    // (handled by `numbers`), and LaTeX commands are expected output.
    let src_nums = numbers(source);
    let cand_nums = numbers(cand);
    if let Some(n) = cand_nums.iter().find(|n| !src_nums.contains(n)) {
        return Err(Violation::NewNumber(n.clone()));
    }
    if !corrected && let Some(n) = src_nums.iter().find(|n| !cand_nums.contains(n)) {
        return Err(Violation::DroppedNumber(n.clone()));
    }

    let src_ids = identifiers(source);
    let cand_ids = identifiers(cand);
    let src_lower: HashSet<String> = src_ids.iter().map(|s| s.to_lowercase()).collect();
    let src_words = words_lower(source);
    for id in &cand_ids {
        let known = src_lower.contains(&id.to_lowercase())
            || src_words.contains(&id.to_lowercase())
            // "pytorch" spoken → "PyTorch" written is fine; so is an id built from spoken parts.
            || src_words.contains(&id.to_lowercase().replace(['_', '-', '.'], ""))
            || id.split(['_', '-', '.']).all(|p| p.is_empty() || src_words.contains(&p.to_lowercase()));
        let latex_ok = mode == Mode::Math && id.starts_with('\\');
        if !known && !latex_ok {
            return Err(Violation::NewIdentifier(id.clone()));
        }
    }
    if !corrected && let Some(id) = src_ids.iter().find(|id| !cand.contains(id.as_str())) {
        return Err(Violation::DroppedIdentifier(id.clone()));
    }

    // Answering instead of rewriting: proper nouns that were never spoken.
    for (i, w) in WORD.find_iter(cand).enumerate() {
        let word = w.as_str();
        let sentence_start = i == 0 || cand[..w.start()].trim_end().ends_with(|c: char| ".!?:\n".contains(c));
        if !sentence_start
            && word.chars().next().is_some_and(char::is_uppercase)
            && word != "I"
            && !src_words.contains(&word.to_lowercase())
        {
            return Err(Violation::NewProperNoun(word.to_string()));
        }
    }

    let content = |s: &str| -> Vec<String> {
        WORD.find_iter(s)
            .map(|m| m.as_str().to_lowercase())
            .filter(|w| w.len() > 3 && !STOPWORDS.contains(&w.as_str()))
            .collect()
    };
    let cand_content = content(cand);
    let new = cand_content.iter().filter(|w| !src_words.contains(*w) && !stem_known(w, &src_words)).count();
    let allowance = match mode {
        Mode::Professional | Mode::Concise => 0.5,
        _ => 0.34,
    };
    if cand_content.len() >= 4 && new as f32 / cand_content.len() as f32 > allowance {
        return Err(Violation::TooMuchNewContent { new, total: cand_content.len() });
    }

    let (before, after) = (word_count(source), word_count(cand));
    let max = before as f32 * 1.6 + 8.0;
    let min = if corrected || mode == Mode::Concise { 0.2 } else { 0.4 } * before as f32;
    if (after as f32) > max || (before >= 6 && (after as f32) < min) {
        return Err(Violation::LengthRatio { before, after });
    }
    Ok(())
}

/// "scheduled" vs "schedule", "tests" vs "test": tolerate inflection changes.
fn stem_known(w: &str, src: &HashSet<String>) -> bool {
    let stem = |s: &str| -> String {
        let s = s.trim_end_matches("'s");
        for suf in ["ing", "ed", "es", "s", "ly"] {
            if s.len() > suf.len() + 3
                && let Some(x) = s.strip_suffix(suf)
            {
                return x.to_string();
            }
        }
        s.to_string()
    };
    let ws = stem(w);
    src.iter().any(|s| stem(s) == ws)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(raw: &str, cand: &str, mode: Mode) -> Result<(), Violation> {
        check(raw, raw, cand, mode)
    }

    #[test]
    fn accepts_faithful_cleanup() {
        assert_eq!(
            ok(
                "so um we need to like refactor the load_config function before friday",
                "We need to refactor the load_config function before Friday.",
                Mode::Clean
            ),
            Ok(())
        );
    }

    #[test]
    fn blueprint_self_correction_may_drop_the_corrected_number() {
        let raw = "Let's schedule the sync for Tuesday at 2 PM, actually no, make it Wednesday at 4:30 PM.";
        assert_eq!(ok(raw, "Let's schedule the sync for Wednesday at 4:30 PM.", Mode::Clean), Ok(()));
    }

    #[test]
    fn rejects_altered_numbers() {
        let raw = "The batch size is 32 and the learning rate is 0.001.";
        assert_eq!(
            ok(raw, "The batch size is 64 and the learning rate is 0.001.", Mode::Clean),
            Err(Violation::NewNumber("64".into()))
        );
        assert_eq!(
            ok(raw, "The batch size and the learning rate are set.", Mode::Clean),
            Err(Violation::DroppedNumber("32".into()))
        );
    }

    #[test]
    fn number_words_and_digits_are_equivalent() {
        assert_eq!(ok("we have two reasons", "We have 2 reasons.", Mode::Clean), Ok(()));
    }

    #[test]
    fn rejects_mangled_identifiers() {
        let raw = "call get_user_by_id from the userService module";
        assert_eq!(
            ok(raw, "Call getUserById from the userService module.", Mode::Technical),
            Err(Violation::NewIdentifier("getUserById".into()))
        );
    }

    #[test]
    fn allows_casing_fixes_of_spoken_terms() {
        assert_eq!(ok("we use pytorch and fastapi", "We use PyTorch and FastAPI.", Mode::Technical), Ok(()));
    }

    #[test]
    fn rejects_answering_the_question() {
        assert!(matches!(
            ok("what is the capital of france", "The capital of France is Paris.", Mode::Clean),
            Err(Violation::NewProperNoun(w)) if w == "Paris"
        ));
        assert_eq!(
            ok("write a haiku about autumn", "Here is a haiku: leaves fall softly down", Mode::Clean),
            Err(Violation::Preamble)
        );
    }

    #[test]
    fn rejects_runaway_expansion() {
        let raw = "fix the bug";
        let cand = "Please fix the bug in the authentication module so that users can log in again without errors appearing on screen every time they try.";
        assert!(ok(raw, cand, Mode::Clean).is_err());
    }

    #[test]
    fn math_mode_allows_latex_commands() {
        let src = r"\sum_{i=1}^{n} x_i";
        assert_eq!(check("sigma from i equals 1 to n of x sub i", src, src, Mode::Math), Ok(()));
        assert_eq!(check("alpha plus beta", "alpha plus beta", r"\alpha + \beta", Mode::Math), Ok(()));
    }

    #[test]
    fn preserves_acronyms() {
        let raw = "Use Reciprocal Rank Fusion over FAISS dense embeddings and BM25 sparse scores.";
        assert_eq!(ok(raw, raw, Mode::Technical), Ok(()));
        assert_eq!(
            ok(raw, "Use Reciprocal Rank Fusion over dense embeddings and BM25 sparse scores.", Mode::Technical),
            Err(Violation::DroppedIdentifier("FAISS".into()))
        );
    }
}
