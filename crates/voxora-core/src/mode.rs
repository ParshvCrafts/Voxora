//! Rewrite modes: what the user wants their speech to become.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Exactly what was said: dictionary fixes and spacing only. Never calls the LLM.
    Verbatim,
    /// Fillers and false starts removed, punctuation and casing fixed.
    Clean,
    /// Clean, then tightened to the fewest words that keep the meaning.
    Concise,
    /// Clean, in a polished business register (email, docs).
    Professional,
    /// Clean, with code identifiers kept intact and spoken math rendered as Unicode.
    Technical,
    /// Spoken math rendered as LaTeX; inline math wrapped in `$...$`.
    Math,
}

impl Mode {
    pub const ALL: [Mode; 6] =
        [Mode::Clean, Mode::Technical, Mode::Math, Mode::Concise, Mode::Professional, Mode::Verbatim];

    /// Short tag shown on the overlay pill.
    pub fn badge(self) -> &'static str {
        match self {
            Mode::Verbatim => "RAW",
            Mode::Clean => "CLEAN",
            Mode::Concise => "BRIEF",
            Mode::Professional => "PRO",
            Mode::Technical => "TECH",
            Mode::Math => "MATH",
        }
    }

    /// Next mode for the quick-cycle hotkey.
    pub fn next(self) -> Mode {
        let i = Mode::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Mode::ALL[(i + 1) % Mode::ALL.len()]
    }

    /// How spoken math should be rendered in this mode, if at all.
    pub fn math_style(self) -> Option<MathStyle> {
        match self {
            Mode::Technical => Some(MathStyle::Unicode),
            Mode::Math => Some(MathStyle::Latex),
            _ => None,
        }
    }

    pub fn uses_llm(self) -> bool {
        self != Mode::Verbatim
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathStyle {
    Unicode,
    Latex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_every_mode_and_wraps() {
        let mut m = Mode::Clean;
        let mut seen = vec![m];
        for _ in 0..Mode::ALL.len() - 1 {
            m = m.next();
            seen.push(m);
        }
        assert_eq!(m.next(), Mode::Clean);
        seen.sort_by_key(|m| m.badge());
        seen.dedup();
        assert_eq!(seen.len(), 6);
    }

    #[test]
    fn serde_names_are_snake_case() {
        assert_eq!(serde_json::to_string(&Mode::Math).unwrap(), "\"math\"");
        let m: Mode = serde_json::from_str("\"technical\"").unwrap();
        assert_eq!(m, Mode::Technical);
    }
}
