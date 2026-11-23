//! Post-ASR guard: drop Whisper's known silence hallucinations and collapse decoding loops.

use std::sync::LazyLock;

use regex::Regex;

static ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\[[A-Z _-]+\]|\((?:music|applause|laughs?|laughter|silence|inaudible|noise|coughs?|sighs?)\)|[♪♫]+",
    )
    .unwrap()
});

/// Phrases Whisper emits on silence/noise (learned from subtitle-heavy training data).
const SILENCE_PHRASES: &[&str] = &[
    "thank you",
    "thanks",
    "thank you for watching",
    "thanks for watching",
    "thank you so much for watching",
    "thank you very much",
    "please subscribe",
    "subscribe to my channel",
    "like and subscribe",
    "you",
    "bye",
    "bye bye",
    "so",
    "okay",
    "subtitles by the amara.org community",
    "transcription by castingwords",
];

/// Clean raw ASR output. Returns `None` when nothing real was said.
///
/// `speech_ms` is how much voiced audio the VAD detected; phrase-level hallucination filtering
/// only applies to near-silent clips, so a user genuinely saying "thank you" still gets it.
pub fn clean(text: &str, speech_ms: u32) -> Option<String> {
    let t = ANNOTATION.replace_all(text, " ");
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if !t.chars().any(char::is_alphanumeric) {
        return None;
    }
    let key: String = t
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .trim()
        .to_string();
    if speech_ms < 700 && SILENCE_PHRASES.contains(&key.as_str()) {
        return None;
    }
    Some(collapse_loops(&t))
}

/// "and then and then and then and then" → "and then". A phrase of 1–8 words repeated 3+
/// times in a row is a decoder loop, not speech.
pub fn collapse_loops(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    let norm = |w: &str| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let mut out: Vec<&str> = Vec::with_capacity(words.len());
    let mut i = 0;
    'outer: while i < words.len() {
        for n in 1..=8 {
            if i + n * 3 > words.len() {
                break;
            }
            let same = |a: usize, b: usize| (0..n).all(|k| norm(words[a + k]) == norm(words[b + k]));
            let mut reps = 1;
            while i + n * (reps + 1) <= words.len() && same(i, i + n * reps) {
                reps += 1;
            }
            if reps >= 3 {
                out.extend_from_slice(&words[i..i + n]);
                i += n * reps;
                continue 'outer;
            }
        }
        out.push(words[i]);
        i += 1;
    }
    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_and_annotation_only_output() {
        assert_eq!(clean("[BLANK_AUDIO]", 0), None);
        assert_eq!(clean("  (music) ♪ ", 0), None);
        assert_eq!(clean("...", 0), None);
    }

    #[test]
    fn rejects_silence_hallucinations_only_when_little_speech() {
        assert_eq!(clean("Thank you for watching.", 120), None);
        assert_eq!(clean(" you", 0), None);
        assert_eq!(clean("Thank you.", 1500).as_deref(), Some("Thank you."));
    }

    #[test]
    fn strips_annotations_inside_speech() {
        assert_eq!(clean("Run the tests [BLANK_AUDIO] now", 2000).as_deref(), Some("Run the tests now"));
    }

    #[test]
    fn collapses_decoder_loops_but_keeps_double_words() {
        assert_eq!(collapse_loops("and then and then and then and then we left"), "and then we left");
        assert_eq!(collapse_loops("no no no no"), "no");
        assert_eq!(collapse_loops("very very good"), "very very good");
    }
}
