//! Spoken math → Unicode or LaTeX, deterministically.
//!
//! "theta transpose x plus b equals zero"   → `θᵀx + b = 0`        (Unicode)
//! "sigma from i equals 1 to n of x sub i"  → `\sum_{i=1}^{n} x_i` (LaTeX)
//!
//! A small recursive-descent parser runs at every word position of the transcript. A span is
//! rewritten only when it parses as *structural* math (an operator, relation, power, fraction,
//! big operator...), so prose like "three times faster" or "beta release" is left untouched:
//! the parser backtracks whenever the right-hand operand is not math.
//!
//! Converted spans are wrapped in private-use sentinels ([`OPEN`]/[`CLOSE`]) so later stages
//! (casing, terminal punctuation, the LLM invariant guard) can tell math from prose.
//! [`finalize`] removes them, wrapping inline LaTeX in `$...$`.

use crate::mode::MathStyle;

pub const OPEN: char = '\u{E000}';
pub const CLOSE: char = '\u{E001}';

// ───────────────────────────── tokens ─────────────────────────────

#[derive(Debug, Clone)]
struct Tok {
    raw: String,
    low: String,
    start: usize,
    end: usize,
    punct: bool,
}

fn tokenize(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut word_start: Option<usize> = None;
    let push_word = |out: &mut Vec<Tok>, s: usize, e: usize| {
        let w = &text[s..e];
        // Peel trailing punctuation off into separate tokens (keeps "2.5" and "4:30" intact).
        let trimmed = w.trim_end_matches(|c: char| ".,;:!?".contains(c));
        let cut = s + trimmed.len();
        if cut > s {
            out.push(Tok { raw: trimmed.to_string(), low: trimmed.to_lowercase(), start: s, end: cut, punct: false });
        }
        for (i, c) in text[cut..e].char_indices() {
            out.push(Tok {
                raw: c.to_string(),
                low: c.to_string(),
                start: cut + i,
                end: cut + i + c.len_utf8(),
                punct: true,
            });
        }
    };
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = word_start.take() {
                push_word(&mut out, s, i);
            }
        } else if word_start.is_none() {
            word_start = Some(i);
        }
    }
    if let Some(s) = word_start {
        push_word(&mut out, s, text.len());
    }
    out
}

// ───────────────────────────── vocabulary ─────────────────────────────

const GREEK: &[(&str, char, Option<char>)] = &[
    ("alpha", 'α', None),
    ("beta", 'β', None),
    ("gamma", 'γ', Some('Γ')),
    ("delta", 'δ', Some('Δ')),
    ("epsilon", 'ε', None),
    ("zeta", 'ζ', None),
    ("eta", 'η', None),
    ("theta", 'θ', Some('Θ')),
    ("iota", 'ι', None),
    ("kappa", 'κ', None),
    ("lambda", 'λ', Some('Λ')),
    ("mu", 'μ', None),
    ("nu", 'ν', None),
    ("xi", 'ξ', Some('Ξ')),
    ("pi", 'π', Some('Π')),
    ("rho", 'ρ', None),
    ("sigma", 'σ', Some('Σ')),
    ("tau", 'τ', None),
    ("upsilon", 'υ', None),
    ("phi", 'φ', Some('Φ')),
    ("chi", 'χ', None),
    ("psi", 'ψ', Some('Ψ')),
    ("omega", 'ω', Some('Ω')),
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
    ("thirteen", "13"),
    ("fourteen", "14"),
    ("fifteen", "15"),
    ("sixteen", "16"),
    ("seventeen", "17"),
    ("eighteen", "18"),
    ("nineteen", "19"),
    ("twenty", "20"),
    ("thirty", "30"),
    ("forty", "40"),
    ("fifty", "50"),
    ("sixty", "60"),
    ("seventy", "70"),
    ("eighty", "80"),
    ("ninety", "90"),
    ("hundred", "100"),
    ("thousand", "1000"),
];

const ORDINALS: &[(&str, &str)] = &[
    ("second", "2"),
    ("third", "3"),
    ("fourth", "4"),
    ("fifth", "5"),
    ("sixth", "6"),
    ("seventh", "7"),
    ("eighth", "8"),
    ("ninth", "9"),
    ("tenth", "10"),
    ("nth", "n"),
    ("kth", "k"),
    ("ith", "i"),
    ("jth", "j"),
];

/// Named functions: spoken word → (LaTeX, plain).
const FUNCS: &[(&str, &str, &str)] = &[
    ("sin", "\\sin", "sin"),
    ("sine", "\\sin", "sin"),
    ("cos", "\\cos", "cos"),
    ("cosine", "\\cos", "cos"),
    ("tan", "\\tan", "tan"),
    ("tangent", "\\tan", "tan"),
    ("tanh", "\\tanh", "tanh"),
    ("log", "\\log", "log"),
    ("ln", "\\ln", "ln"),
    ("exp", "\\exp", "exp"),
    ("max", "\\max", "max"),
    ("min", "\\min", "min"),
    ("softmax", "\\operatorname{softmax}", "softmax"),
    ("sigmoid", "\\sigma", "σ"),
    ("relu", "\\operatorname{ReLU}", "ReLU"),
];

/// Single letters that read naturally as functions: "f of x", "P of A given B", "L of theta".
const FUNC_LETTERS: &str = "fghpqlje";

/// Relations: spoken forms (longest first) → (LaTeX, Unicode).
const RELATIONS: &[(&[&str], &str, &str)] = &[
    (&["is", "not", "equal", "to"], "\\neq", "≠"),
    (&["does", "not", "equal"], "\\neq", "≠"),
    (&["not", "equal", "to"], "\\neq", "≠"),
    (&["not", "equals"], "\\neq", "≠"),
    (&["is", "greater", "than", "or", "equal", "to"], "\\geq", "≥"),
    (&["greater", "than", "or", "equal", "to"], "\\geq", "≥"),
    (&["greater", "than", "or", "equals"], "\\geq", "≥"),
    (&["is", "less", "than", "or", "equal", "to"], "\\leq", "≤"),
    (&["less", "than", "or", "equal", "to"], "\\leq", "≤"),
    (&["less", "than", "or", "equals"], "\\leq", "≤"),
    (&["is", "greater", "than"], ">", ">"),
    (&["greater", "than"], ">", ">"),
    (&["is", "less", "than"], "<", "<"),
    (&["less", "than"], "<", "<"),
    (&["is", "approximately", "equal", "to"], "\\approx", "≈"),
    (&["approximately", "equal", "to"], "\\approx", "≈"),
    (&["approximately", "equals"], "\\approx", "≈"),
    (&["is", "proportional", "to"], "\\propto", "∝"),
    (&["proportional", "to"], "\\propto", "∝"),
    (&["is", "an", "element", "of"], "\\in", "∈"),
    (&["is", "in", "the", "set"], "\\in", "∈"),
    (&["element", "of"], "\\in", "∈"),
    (&["is", "equal", "to"], "=", "="),
    (&["equal", "to"], "=", "="),
    (&["equals"], "=", "="),
    (&["="], "=", "="),
    (&["goes", "to"], "\\to", "→"),
    (&["tends", "to"], "\\to", "→"),
    (&["approaches"], "\\to", "→"),
];

/// Words that keep a math span going; used to decide whether the article "a" is a variable.
const MATH_WORDS: &[&str] = &[
    "plus",
    "minus",
    "times",
    "over",
    "equals",
    "squared",
    "cubed",
    "sub",
    "to",
    "transpose",
    "divided",
    "greater",
    "less",
    "dot",
    "prime",
    "inverse",
    "factorial",
    "hat",
    "bar",
    "is",
    "given",
];

// ───────────────────────────── AST ─────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(String),
    Var(String),
    Greek(&'static str, char),
    Const(&'static str, &'static str),
    Neg(Box<Expr>),
    Bin(&'static str, &'static str, Box<Expr>, Box<Expr>),
    Rel(&'static str, &'static str, Box<Expr>, Box<Expr>),
    Frac(Box<Expr>, Box<Expr>),
    Juxt(Vec<Expr>),
    Sup(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Accent(&'static str, char, Box<Expr>),
    Postfix(Box<Expr>, &'static str),
    Func(String, String, Box<Expr>),
    Sqrt(Box<Expr>),
    Abs(Box<Expr>),
    Paren(Box<Expr>),
    BigOp {
        latex: &'static str,
        uni: &'static str,
        var: Option<Box<Expr>>,
        lower: Option<Box<Expr>>,
        upper: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Lim {
        var: Box<Expr>,
        to: Box<Expr>,
        body: Box<Expr>,
    },
    Diff(Box<Expr>),
    Partial(Box<Expr>),
}

fn b(e: Expr) -> Box<Expr> {
    Box::new(e)
}

impl Expr {
    /// True when the expression is real math rather than a lone word or number.
    fn is_structural(&self, lone_greek: bool) -> bool {
        match self {
            Expr::Num(_) | Expr::Var(_) | Expr::Const(..) => false,
            Expr::Greek(..) => lone_greek,
            Expr::Juxt(items) => items.iter().any(|e| e.is_structural(lone_greek)),
            Expr::Neg(inner) | Expr::Paren(inner) => inner.is_structural(lone_greek),
            _ => true,
        }
    }

    fn is_atom(&self) -> bool {
        matches!(self, Expr::Num(_) | Expr::Var(_) | Expr::Greek(..) | Expr::Const(..))
    }
}

// ───────────────────────────── parser ─────────────────────────────

#[derive(Clone, Copy)]
struct Ctx {
    /// Allow "f of x" function application (off inside big-operator bounds).
    of: bool,
    /// Allow "given" as a relation (inside P(...) / E(...)).
    given: bool,
}

const TOP: Ctx = Ctx { of: true, given: false };
const BOUND: Ctx = Ctx { of: false, given: false };

struct Parser<'a> {
    t: &'a [Tok],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek_at(&self, k: usize) -> Option<&'a Tok> {
        self.t.get(self.pos + k).filter(|t| !t.punct)
    }

    fn peek(&self) -> Option<&'a str> {
        self.peek_at(0).map(|t| t.low.as_str())
    }

    fn eat(&mut self, words: &[&str]) -> bool {
        let ok = words.iter().enumerate().all(|(k, w)| self.peek_at(k).is_some_and(|t| t.low == *w));
        if ok {
            self.pos += words.len();
        }
        ok
    }

    fn eat_any(&mut self, alts: &[&[&str]]) -> bool {
        alts.iter().any(|a| self.eat(a))
    }

    /// Run `f`; on failure rewind so the caller can try something else.
    fn attempt<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let save = self.pos;
        let r = f(self);
        if r.is_none() {
            self.pos = save;
        }
        r
    }

    fn relation(&mut self, ctx: Ctx) -> Option<Expr> {
        let mut left = self.sum(ctx)?;
        loop {
            let save = self.pos;
            let op = if ctx.given && self.eat(&["given"]) {
                Some(("\\mid", "|"))
            } else {
                RELATIONS.iter().find(|(w, _, _)| self.eat(w)).map(|(_, l, u)| (*l, *u))
            };
            let Some((l, u)) = op else { break };
            match self.sum(ctx) {
                Some(right) => left = Expr::Rel(l, u, b(left), b(right)),
                None => {
                    self.pos = save;
                    break;
                }
            }
        }
        Some(left)
    }

    fn sum(&mut self, ctx: Ctx) -> Option<Expr> {
        let mut left = self.term(ctx)?;
        loop {
            let save = self.pos;
            let op = if self.eat(&["plus", "or", "minus"]) {
                ("\\pm", "±")
            } else if self.eat_any(&[&["plus"], &["+"]]) {
                ("+", "+")
            } else if self.eat_any(&[&["minus"], &["-"]]) {
                ("-", "-")
            } else {
                break;
            };
            match self.term(ctx) {
                Some(right) => left = Expr::Bin(op.0, op.1, b(left), b(right)),
                None => {
                    self.pos = save;
                    break;
                }
            }
        }
        Some(left)
    }

    fn term(&mut self, ctx: Ctx) -> Option<Expr> {
        let mut left = self.juxt(ctx)?;
        loop {
            let save = self.pos;
            enum Op {
                Mul(&'static str, &'static str),
                Frac,
            }
            let op = if self.eat_any(&[&["times"], &["multiplied", "by"], &["*"], &["×"]]) {
                Op::Mul("\\times", "×")
            } else if self.eat(&["dot"]) {
                Op::Mul("\\cdot", "·")
            } else if self.eat_any(&[&["over"], &["divided", "by"], &["/"]]) {
                Op::Frac
            } else {
                break;
            };
            match self.juxt(ctx) {
                Some(right) => {
                    left = match op {
                        Op::Mul(l, u) => Expr::Bin(l, u, b(left), b(right)),
                        Op::Frac => Expr::Frac(b(left), b(right)),
                    }
                }
                None => {
                    self.pos = save;
                    break;
                }
            }
        }
        Some(left)
    }

    /// Implicit multiplication: "theta transpose x", "2 pi r", "d x".
    fn juxt(&mut self, ctx: Ctx) -> Option<Expr> {
        let mut items = Vec::new();
        let mut push = |f: Expr| match f {
            Expr::Juxt(inner) => items.extend(inner), // a "dx" token is two factors
            f => items.push(f),
        };
        push(self.factor(ctx)?);
        // "x minus 1" is subtraction, never x·(-1).
        while !matches!(self.peek(), Some("minus" | "negative" | "-"))
            && let Some(f) = self.attempt(|p| p.factor(ctx))
        {
            push(f);
        }
        Some(if items.len() == 1 { items.pop().unwrap() } else { Expr::Juxt(items) })
    }

    fn factor(&mut self, ctx: Ctx) -> Option<Expr> {
        let mut e = self.primary(ctx)?;
        loop {
            if self.eat(&["squared"]) {
                e = Expr::Sup(b(e), b(Expr::Num("2".into())));
            } else if self.eat(&["cubed"]) {
                e = Expr::Sup(b(e), b(Expr::Num("3".into())));
            } else if self.eat_any(&[&["transpose"], &["transposed"]]) {
                e = Expr::Sup(b(e), b(Expr::Var("T".into())));
            } else if self.eat(&["inverse"]) {
                e = Expr::Sup(b(e), b(Expr::Neg(b(Expr::Num("1".into())))));
            } else if self.eat(&["prime"]) {
                e = Expr::Postfix(b(e), "'");
            } else if self.eat(&["factorial"]) {
                e = Expr::Postfix(b(e), "!");
            } else if self.eat(&["hat"]) {
                e = Expr::Accent("\\hat", '\u{0302}', b(e));
            } else if self.eat(&["bar"]) {
                e = Expr::Accent("\\bar", '\u{0304}', b(e));
            } else if self.eat(&["tilde"]) {
                e = Expr::Accent("\\tilde", '\u{0303}', b(e));
            } else if let Some(exp) = self.attempt(|p| {
                let ok = p.eat_any(&[
                    &["to", "the", "power", "of"],
                    &["raised", "to", "the", "power", "of"],
                    &["raised", "to", "the"],
                    &["raised", "to"],
                    &["to", "the"],
                ]);
                if !ok {
                    return None;
                }
                if let Some(t) = p.peek().and_then(|w| lookup(ORDINALS, w)) {
                    p.pos += 1;
                    return Some(atom_from(t));
                }
                p.primary(BOUND)
            }) {
                e = Expr::Sup(b(e), b(exp));
            } else if let Some(sub) = self.attempt(|p| {
                if !p.eat_any(&[&["sub"], &["subscript"]]) {
                    return None;
                }
                p.index_atom()
            }) {
                e = Expr::Sub(b(e), b(sub));
            } else {
                break;
            }
        }
        Some(e)
    }

    /// Operand of "sub" / a summation variable: a letter (including "I"), number or Greek letter.
    fn index_atom(&mut self) -> Option<Expr> {
        let t = self.peek_at(0)?;
        if is_letter(&t.raw) {
            self.pos += 1;
            return Some(Expr::Var(t.raw.to_lowercase_if_i()));
        }
        self.primary(BOUND)
    }

    fn primary(&mut self, ctx: Ctx) -> Option<Expr> {
        let t = self.peek_at(0)?;
        let w = t.low.as_str();

        if self.eat_any(&[&["minus"], &["negative"], &["-"]]) {
            return self.attempt(|p| p.factor(ctx)).map(|e| Expr::Neg(b(e)));
        }
        if let Some(e) = self.attempt(|p| {
            p.eat(&["the"]);
            p.structure(ctx)
        }) {
            return Some(e);
        }
        if self.eat_any(&[&["open", "paren"], &["left", "paren"], &["open", "parenthesis"]]) {
            let inner = self.relation(ctx)?;
            self.eat_any(&[&["close", "paren"], &["right", "paren"], &["close", "parenthesis"]]);
            return Some(Expr::Paren(b(inner)));
        }
        if self.eat(&["infinity"]) {
            return Some(Expr::Const("\\infty", "∞"));
        }
        if self.eat_any(&[&["nabla"], &["del"]]) {
            return Some(Expr::Const("\\nabla", "∇"));
        }
        if self.eat(&["partial"]) {
            return self.primary(ctx).map(|e| Expr::Partial(b(e)));
        }
        // Greek, optionally "capital sigma".
        let capital = self.peek() == Some("capital") || self.peek() == Some("big");
        let gw = if capital { self.peek_at(1).map(|t| t.low.as_str()) } else { Some(w) };
        if let Some((name, lo, up)) = gw.and_then(|g| GREEK.iter().find(|(n, ..)| *n == g)) {
            if capital && up.is_none() {
                return None;
            }
            self.pos += if capital { 2 } else { 1 };
            return Some(match (capital, up) {
                (true, Some(u)) => Expr::Greek(capital_name(name), *u),
                _ => Expr::Greek(name, *lo),
            });
        }
        // Named function: "sine of theta", "log x".
        if let Some((_, l, u)) = FUNCS.iter().find(|(n, ..)| *n == w) {
            return self.attempt(|p| {
                p.pos += 1;
                p.eat(&["of"]);
                let arg = p.factor(ctx)?;
                Some(Expr::Func(l.to_string(), u.to_string(), b(arg)))
            });
        }
        // Number (digits or a number word), with "2 point 5" decimals.
        if let Some(n) = number(t) {
            self.pos += 1;
            if let Some(frac) = self.attempt(|p| {
                if !p.eat(&["point"]) {
                    return None;
                }
                let d = p.peek_at(0).and_then(number)?;
                p.pos += 1;
                Some(d)
            }) {
                return Some(Expr::Num(format!("{n}.{frac}")));
            }
            return Some(Expr::Num(n));
        }
        // "capital A": the only way to get an uppercase variable, because ASR capitalizes
        // single letters arbitrarily ("Theta transpose X plus B equals zero").
        if matches!(w, "capital" | "big" | "uppercase")
            && let Some(n) = self.peek_at(1).filter(|n| is_letter(&n.raw))
        {
            self.pos += 2;
            return Some(Expr::Var(n.raw.to_uppercase()));
        }
        // "dx", "df": ASR often writes a differential as one token.
        if w.len() == 2 && w.starts_with('d') && w[1..].chars().all(|c| c.is_ascii_lowercase() && !"aeiou".contains(c))
        {
            self.pos += 1;
            return Some(Expr::Juxt(vec![Expr::Var("d".into()), Expr::Var(w[1..].into())]));
        }
        // Single-letter variable or function.
        if is_letter(&t.raw) && t.raw != "I" {
            let next = self.peek_at(1).map(|t| t.low.as_str());
            if w == "a" && !next.is_none_or(|n| MATH_WORDS.contains(&n)) {
                return None; // the article "a", not a variable
            }
            if ctx.of && FUNC_LETTERS.contains(w) && next == Some("of") {
                let raw = t.raw.clone();
                return self
                    .attempt(|p| {
                        p.pos += 2;
                        if w == "p" || w == "e" {
                            let arg = p.relation(Ctx { of: true, given: true })?;
                            // Conditional probability reads as P(Y = 1 | X) by convention.
                            if matches!(arg, Expr::Rel("\\mid", ..)) {
                                let name = w.to_uppercase();
                                return Some(Expr::Func(name.clone(), name, b(uppercase_vars(arg))));
                            }
                            return Some(Expr::Func(raw.clone(), raw.clone(), b(arg)));
                        }
                        let arg = p.factor(ctx)?;
                        Some(Expr::Func(raw.clone(), raw.clone(), b(arg)))
                    })
                    .or_else(|| {
                        self.pos += 1;
                        Some(Expr::Var(w.to_string()))
                    });
            }
            self.pos += 1;
            return Some(Expr::Var(w.to_string()));
        }
        None
    }

    /// Big operators and other multi-word constructs.
    fn structure(&mut self, ctx: Ctx) -> Option<Expr> {
        if self.eat_any(&[&["square", "root", "of"], &["root", "of"], &["sqrt", "of"]]) {
            return self.factor(ctx).map(|e| Expr::Sqrt(b(e)));
        }
        if self.eat(&["absolute", "value", "of"]) {
            return self.factor(ctx).map(|e| Expr::Abs(b(e)));
        }
        if self.eat_any(&[&["gradient", "of"], &["grad", "of"]]) {
            return self.factor(ctx).map(|e| Expr::Juxt(vec![Expr::Const("\\nabla", "∇"), e]));
        }
        if self.eat_any(&[&["limit"], &["lim"]]) {
            self.eat(&["as"]);
            let var = self.index_atom()?;
            if !self.eat_any(&[&["approaches"], &["goes", "to"], &["tends", "to"], &["to"]]) {
                return None;
            }
            let to = self.sum(BOUND)?;
            self.eat(&["of"]);
            let body = self.term(ctx)?;
            return Some(Expr::Lim { var: b(var), to: b(to), body: b(body) });
        }
        let next_is_bound = |p: &Self| matches!(p.peek_at(1).map(|t| t.low.as_str()), Some("from" | "over" | "of"));
        let (latex, uni, has_var) = if self.eat_any(&[&["sum"], &["summation"]])
            || (self.peek() == Some("sigma") && next_is_bound(self) && self.eat(&["sigma"]))
        {
            ("\\sum", "∑", true)
        } else if self.peek() == Some("product") && next_is_bound(self) && self.eat(&["product"]) {
            ("\\prod", "∏", true)
        } else if self.eat(&["integral"]) {
            ("\\int", "∫", false)
        } else {
            return None;
        };
        let (mut var, mut lower, mut upper) = (None, None, None);
        if self.eat(&["from"]) {
            if has_var {
                var = Some(b(self.index_atom()?));
                if !self.eat_any(&[&["equals"], &["equal", "to"], &["="], &["is"]]) {
                    return None;
                }
            }
            lower = Some(b(self.sum(BOUND)?));
            if !self.eat(&["to"]) {
                return None;
            }
            upper = Some(b(self.sum(BOUND)?));
        } else if has_var && self.eat(&["over"]) {
            var = Some(b(self.index_atom()?));
        }
        self.eat(&["of"]);
        let body = self.sum(ctx)?;
        let body = if latex == "\\int" { with_differential(body) } else { body };
        Some(Expr::BigOp { latex, uni, var, lower, upper, body: b(body) })
    }
}

trait LowerI {
    fn to_lowercase_if_i(&self) -> String;
}
impl LowerI for String {
    /// ASR writes the index "i" as the pronoun "I"; inside indices it is always lowercase.
    fn to_lowercase_if_i(&self) -> String {
        if self == "I" { "i".into() } else { self.clone() }
    }
}

fn is_letter(s: &str) -> bool {
    s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic())
}

fn number(t: &Tok) -> Option<String> {
    let w = t.low.as_str();
    let digits = !w.is_empty()
        && w.chars().next().is_some_and(|c| c.is_ascii_digit())
        && w.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ':');
    if digits {
        return Some(w.to_string());
    }
    lookup(NUMBER_WORDS, w).map(str::to_string)
}

fn lookup(table: &[(&'static str, &'static str)], w: &str) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == w).map(|(_, v)| *v)
}

fn atom_from(s: &str) -> Expr {
    if s.chars().all(|c| c.is_ascii_digit()) { Expr::Num(s.into()) } else { Expr::Var(s.into()) }
}

fn capital_name(name: &str) -> &'static str {
    match name {
        "gamma" => "Gamma",
        "delta" => "Delta",
        "theta" => "Theta",
        "lambda" => "Lambda",
        "xi" => "Xi",
        "pi" => "Pi",
        "sigma" => "Sigma",
        "phi" => "Phi",
        "psi" => "Psi",
        _ => "Omega",
    }
}

fn uppercase_vars(e: Expr) -> Expr {
    let up = |x: Box<Expr>| b(uppercase_vars(*x));
    match e {
        Expr::Var(v) => Expr::Var(v.to_uppercase()),
        Expr::Rel(l, u, a, c) => Expr::Rel(l, u, up(a), up(c)),
        Expr::Bin(l, u, a, c) => Expr::Bin(l, u, up(a), up(c)),
        Expr::Juxt(items) => Expr::Juxt(items.into_iter().map(uppercase_vars).collect()),
        Expr::Neg(x) => Expr::Neg(up(x)),
        Expr::Paren(x) => Expr::Paren(up(x)),
        other => other,
    }
}

/// "integral of x squared d x": the trailing `d x` is a differential, not d times x.
fn with_differential(body: Expr) -> Expr {
    if let Expr::Juxt(mut items) = body {
        let n = items.len();
        if n >= 2 && items[n - 2] == Expr::Var("d".into()) && matches!(items[n - 1], Expr::Var(_)) {
            let v = items.pop().unwrap();
            items.pop();
            items.push(Expr::Diff(b(v)));
        }
        return if items.len() == 1 { items.pop().unwrap() } else { Expr::Juxt(items) };
    }
    body
}

// ───────────────────────────── rendering ─────────────────────────────

fn superscript(s: &str) -> Option<String> {
    s.chars()
        .map(|c| {
            Some(match c {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                '+' => '⁺',
                '-' => '⁻',
                '=' => '⁼',
                '(' => '⁽',
                ')' => '⁾',
                'n' => 'ⁿ',
                'i' => 'ⁱ',
                'T' => 'ᵀ',
                'a' => 'ᵃ',
                'b' => 'ᵇ',
                'c' => 'ᶜ',
                'd' => 'ᵈ',
                'e' => 'ᵉ',
                'f' => 'ᶠ',
                'g' => 'ᵍ',
                'h' => 'ʰ',
                'j' => 'ʲ',
                'k' => 'ᵏ',
                'l' => 'ˡ',
                'm' => 'ᵐ',
                'o' => 'ᵒ',
                'p' => 'ᵖ',
                'r' => 'ʳ',
                's' => 'ˢ',
                't' => 'ᵗ',
                'u' => 'ᵘ',
                'v' => 'ᵛ',
                'w' => 'ʷ',
                'x' => 'ˣ',
                'y' => 'ʸ',
                'z' => 'ᶻ',
                ' ' => ' ',
                _ => return None,
            })
        })
        .collect()
}

fn subscript(s: &str) -> Option<String> {
    s.chars()
        .map(|c| {
            Some(match c {
                '0' => '₀',
                '1' => '₁',
                '2' => '₂',
                '3' => '₃',
                '4' => '₄',
                '5' => '₅',
                '6' => '₆',
                '7' => '₇',
                '8' => '₈',
                '9' => '₉',
                '+' => '₊',
                '-' => '₋',
                '=' => '₌',
                '(' => '₍',
                ')' => '₎',
                'a' => 'ₐ',
                'e' => 'ₑ',
                'o' => 'ₒ',
                'x' => 'ₓ',
                'h' => 'ₕ',
                'k' => 'ₖ',
                'l' => 'ₗ',
                'm' => 'ₘ',
                'n' => 'ₙ',
                'p' => 'ₚ',
                's' => 'ₛ',
                't' => 'ₜ',
                'i' => 'ᵢ',
                'j' => 'ⱼ',
                'r' => 'ᵣ',
                'u' => 'ᵤ',
                'v' => 'ᵥ',
                _ => return None,
            })
        })
        .collect()
}

fn needs_parens(e: &Expr) -> bool {
    matches!(e, Expr::Bin(..) | Expr::Rel(..) | Expr::Frac(..) | Expr::Neg(_) | Expr::Juxt(_))
}

fn uni(e: &Expr) -> String {
    let wrap = |e: &Expr| {
        let s = uni(e);
        if needs_parens(e) { format!("({s})") } else { s }
    };
    match e {
        Expr::Num(n) | Expr::Var(n) => n.clone(),
        Expr::Greek(_, c) => c.to_string(),
        Expr::Const(_, u) => u.to_string(),
        Expr::Neg(x) => format!("-{}", wrap(x)),
        Expr::Bin(_, op, l, r) | Expr::Rel(_, op, l, r) => format!("{} {op} {}", uni(l), uni(r)),
        Expr::Frac(n, d) => format!("{}/{}", wrap(n), wrap(d)),
        Expr::Juxt(items) => {
            let mut s = String::new();
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    let prev = &items[i - 1];
                    let num_num = matches!(prev, Expr::Num(_)) && matches!(it, Expr::Num(_));
                    if num_num || matches!(it, Expr::Diff(_)) {
                        s.push(' ');
                    }
                }
                s.push_str(&uni(it));
            }
            s
        }
        Expr::Sup(base, exp) => {
            let x = uni(exp);
            match superscript(&x) {
                Some(sup) => format!("{}{sup}", wrap(base)),
                None if x.chars().count() == 1 => format!("{}^{x}", wrap(base)),
                None => format!("{}^({x})", wrap(base)),
            }
        }
        Expr::Sub(base, idx) => {
            let x = uni(idx);
            match subscript(&x) {
                Some(sub) => format!("{}{sub}", wrap(base)),
                None => format!("{}_{x}", wrap(base)),
            }
        }
        Expr::Accent(_, c, x) => format!("{}{c}", wrap(x)),
        Expr::Postfix(x, p) => format!("{}{p}", wrap(x)),
        Expr::Func(_, name, arg) => format!("{name}({})", uni(arg)),
        Expr::Sqrt(x) => {
            if x.is_atom() {
                format!("√{}", uni(x))
            } else {
                format!("√({})", uni(x))
            }
        }
        Expr::Abs(x) => format!("|{}|", uni(x)),
        Expr::Paren(x) => format!("({})", uni(x)),
        Expr::BigOp { uni: sym, var, lower, upper, body, .. } => {
            let lo = match (var, lower) {
                (Some(v), Some(l)) => Some(format!("{}={}", uni(v), uni(l))),
                (Some(v), None) => Some(uni(v)),
                (None, Some(l)) => Some(uni(l)),
                (None, None) => None,
            };
            let hi = upper.as_ref().map(|u| uni(u));
            let pretty_lo = lo.as_deref().map(subscript);
            let pretty_hi = hi.as_deref().map(superscript);
            let limits = match (pretty_lo, pretty_hi) {
                (Some(Some(l)), Some(Some(h))) => format!("{l}{h}"),
                (Some(Some(l)), None) => l,
                _ => {
                    let mut s = String::new();
                    if let Some(l) = &lo {
                        s += &format!("_{{{l}}}");
                    }
                    if let Some(h) = &hi {
                        s += &format!("^{{{h}}}");
                    }
                    s
                }
            };
            format!("{sym}{limits} {}", uni(body))
        }
        Expr::Lim { var, to, body } => format!("lim_{{{}→{}}} {}", uni(var), uni(to), uni(body)),
        Expr::Diff(v) => format!("d{}", uni(v)),
        Expr::Partial(x) => format!("∂{}", uni(x)),
    }
}

fn group(s: String) -> String {
    if s.chars().count() == 1 { s } else { format!("{{{s}}}") }
}

fn tex(e: &Expr) -> String {
    let wrap = |e: &Expr| {
        let s = tex(e);
        if needs_parens(e) { format!("({s})") } else { s }
    };
    match e {
        Expr::Num(n) | Expr::Var(n) => n.clone(),
        Expr::Greek(name, _) => format!("\\{name}"),
        Expr::Const(l, _) => l.to_string(),
        Expr::Neg(x) => format!("-{}", wrap(x)),
        Expr::Bin(op, _, l, r) | Expr::Rel(op, _, l, r) => format!("{} {op} {}", tex(l), tex(r)),
        Expr::Frac(n, d) => format!("\\frac{{{}}}{{{}}}", tex(n), tex(d)),
        Expr::Juxt(items) => {
            let mut s = String::new();
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    let prev = &items[i - 1];
                    let tight = matches!(prev, Expr::Num(_) | Expr::Var(_)) && matches!(it, Expr::Var(_))
                        || matches!(prev, Expr::Num(_)) && !matches!(it, Expr::Num(_) | Expr::Diff(_));
                    if !tight {
                        s.push(' ');
                    }
                }
                s.push_str(&tex(it));
            }
            s
        }
        Expr::Sup(base, exp) => format!("{}^{}", wrap(base), group(tex(exp))),
        Expr::Sub(base, idx) => format!("{}_{}", wrap(base), group(tex(idx))),
        Expr::Accent(cmd, _, x) => format!("{cmd}{{{}}}", tex(x)),
        Expr::Postfix(x, p) => format!("{}{p}", wrap(x)),
        Expr::Func(name, _, arg) => format!("{name}({})", tex(arg)),
        Expr::Sqrt(x) => format!("\\sqrt{{{}}}", tex(x)),
        Expr::Abs(x) => format!("\\lvert {} \\rvert", tex(x)),
        Expr::Paren(x) => format!("({})", tex(x)),
        Expr::BigOp { latex, var, lower, upper, body, .. } => {
            let mut s = latex.to_string();
            match (var, lower) {
                (Some(v), Some(l)) => s += &format!("_{{{}={}}}", tex(v), tex(l)),
                (Some(v), None) => s += &format!("_{{{}}}", tex(v)),
                (None, Some(l)) => s += &format!("_{{{}}}", tex(l)),
                (None, None) => {}
            }
            if let Some(u) = upper {
                s += &format!("^{{{}}}", tex(u));
            }
            format!("{s} {}", tex(body))
        }
        Expr::Lim { var, to, body } => format!("\\lim_{{{} \\to {}}} {}", tex(var), tex(to), tex(body)),
        Expr::Diff(v) => format!("\\, d{}", tex(v)),
        Expr::Partial(x) => format!("\\partial {}", tex(x)),
    }
}

// ───────────────────────────── public API ─────────────────────────────

/// Replace spoken-math spans in `text` with rendered math wrapped in [`OPEN`]/[`CLOSE`].
/// Everything outside the spans is copied byte-for-byte.
pub fn convert(text: &str, style: MathStyle) -> String {
    let toks = tokenize(text);
    let lone_greek = style == MathStyle::Latex;
    let mut out = String::with_capacity(text.len() + 8);
    let mut copied_to = 0;
    let mut i = 0;
    while i < toks.len() {
        if toks[i].punct {
            i += 1;
            continue;
        }
        let mut p = Parser { t: &toks, pos: i };
        let parsed = p.relation(TOP).filter(|e| e.is_structural(lone_greek));
        match parsed {
            Some(e) if p.pos > i && (p.pos - i >= 2 || lone_greek) => {
                let (s, end) = (toks[i].start, toks[p.pos - 1].end);
                out.push_str(&text[copied_to..s]);
                out.push(OPEN);
                out.push_str(&match style {
                    MathStyle::Unicode => uni(&e),
                    MathStyle::Latex => tex(&e),
                });
                out.push(CLOSE);
                copied_to = end;
                i = p.pos;
            }
            _ => i += 1,
        }
    }
    out.push_str(&text[copied_to..]);
    out
}

/// True when the text (ignoring trailing punctuation) is a single math span.
pub fn is_whole_math(text: &str) -> bool {
    let t = text.trim().trim_end_matches(|c: char| ".,;:!?".contains(c)).trim_end();
    t.starts_with(OPEN) && t.ends_with(CLOSE) && t.matches(OPEN).count() == 1
}

/// Remove sentinels. LaTeX spans embedded in prose become `$...$`; a whole-math utterance
/// stays bare and loses any terminal period.
pub fn finalize(text: &str, style: MathStyle) -> String {
    if !text.contains(OPEN) {
        return text.to_string();
    }
    if is_whole_math(text) {
        let t = text.trim().trim_end_matches(|c: char| ".,;:!?".contains(c)).trim_end();
        return t.replace([OPEN, CLOSE], "");
    }
    match style {
        MathStyle::Latex => text.replace([OPEN, CLOSE], "$"),
        MathStyle::Unicode => text.replace([OPEN, CLOSE], ""),
    }
}

/// Strip sentinels without any style decisions (used when math was not requested).
pub fn strip(text: &str) -> String {
    text.replace([OPEN, CLOSE], "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use MathStyle::{Latex, Unicode};

    fn u(s: &str) -> String {
        finalize(&convert(s, Unicode), Unicode)
    }
    fn l(s: &str) -> String {
        finalize(&convert(s, Latex), Latex)
    }

    // Blueprint test-matrix cases.
    #[test]
    fn blueprint_theta_transpose_unicode() {
        assert_eq!(u("theta transpose x plus b equals zero"), "θᵀx + b = 0");
        assert_eq!(u("Theta transpose x plus b equals zero."), "θᵀx + b = 0");
    }

    #[test]
    fn blueprint_summation_latex() {
        assert_eq!(l("sigma from i equals 1 to n of x sub i"), r"\sum_{i=1}^{n} x_i");
        assert_eq!(l("Sigma from I equals one to n of x sub I."), r"\sum_{i=1}^{n} x_i");
    }

    #[test]
    fn summation_unicode_uses_real_sub_and_superscripts() {
        assert_eq!(u("sum from i equals 1 to n of x sub i"), "∑ᵢ₌₁ⁿ xᵢ");
    }

    #[test]
    fn powers_and_relations() {
        assert_eq!(u("x squared plus y squared equals z squared"), "x² + y² = z²");
        assert_eq!(l("x squared plus y squared equals z squared"), "x^2 + y^2 = z^2");
        assert_eq!(u("e to the power of n minus 1"), "eⁿ - 1");
        assert_eq!(u("a is less than or equal to b"), "a ≤ b");
        assert_eq!(l("x is not equal to 0"), r"x \neq 0");
        assert_eq!(u("x to the nth"), "xⁿ");
        assert_eq!(l("capital A inverse"), "A^{-1}");
    }

    #[test]
    fn asr_casing_and_token_quirks() {
        // Whisper capitalizes single letters at random; variables are lowercase unless "capital".
        assert_eq!(u("Theta transpose X plus B equals zero."), "θᵀx + b = 0");
        assert_eq!(l("p of y equals 1 given x."), r"P(Y = 1 \mid X)");
        assert_eq!(l("The derivative df over dx is positive."), r"The derivative $\frac{df}{dx}$ is positive.");
        assert_eq!(l("Integral from 0 to 1 of x squared dx."), r"\int_{0}^{1} x^2 \, dx");
        assert_eq!(u("Do it now"), "Do it now", "'do' is a word, not a differential");
    }

    #[test]
    fn fractions_derivatives_and_calculus() {
        assert_eq!(l("d f over d x"), r"\frac{df}{dx}");
        assert_eq!(l("partial f over partial x"), r"\frac{\partial f}{\partial x}");
        assert_eq!(l("integral from 0 to 1 of x squared d x"), r"\int_{0}^{1} x^2 \, dx");
        assert_eq!(u("integral from 0 to 1 of x squared d x"), "∫₀¹ x² dx");
        assert_eq!(l("the limit as x approaches 0 of sine of x over x"), r"\lim_{x \to 0} \frac{\sin(x)}{x}");
        assert_eq!(l("square root of x plus 1"), r"\sqrt{x} + 1");
    }

    #[test]
    fn probability_with_given() {
        assert_eq!(l("P of Y equals 1 given X"), r"P(Y = 1 \mid X)");
        assert_eq!(u("P of A given B"), "P(A | B)");
    }

    #[test]
    fn greek_and_capitals() {
        assert_eq!(u("alpha plus beta"), "α + β");
        assert_eq!(l("capital sigma inverse"), r"\Sigma^{-1}");
        assert_eq!(l("alpha"), r"\alpha");
        assert_eq!(u("2 pi r"), "2 pi r", "lone juxtaposition is not structural");
        assert_eq!(u("2 pi r squared"), "2πr²");
    }

    #[test]
    fn inline_math_in_prose() {
        assert_eq!(
            l("The loss is x squared plus y squared, minimized over w."),
            "The loss is $x^2 + y^2$, minimized over w."
        );
        assert_eq!(
            u("so we set theta transpose x plus b equals zero for the boundary"),
            "so we set θᵀx + b = 0 for the boundary"
        );
    }

    #[test]
    fn prose_is_left_alone() {
        // Math mode deliberately renders a lone Greek name ("where alpha is the learning rate").
        assert_eq!(u("Ship the beta release on Friday."), "Ship the beta release on Friday.");
        assert_eq!(l("where alpha is the rate"), r"where $\alpha$ is the rate");
        for s in [
            "It runs three times faster than before.",
            "Plus we need to update the docs.",
            "Add a function that returns the sum.",
            "We went from Boston to New York.",
            "I think a lot about this.",
            "The integral part of the plan is testing.",
            "Give me two to three hours.",
            "Take the square of the problem seriously.",
        ] {
            assert_eq!(u(s), s, "Unicode altered prose: {s}");
            assert_eq!(l(s), s, "LaTeX altered prose: {s}");
        }
    }

    #[test]
    fn arithmetic_words() {
        assert_eq!(u("one plus one equals two"), "1 + 1 = 2");
        assert_eq!(u("3 times 4"), "3 × 4");
        assert_eq!(u("x plus or minus 2 point 5"), "x ± 2.5");
    }
}
