//! Zero-loss history: every utterance is committed to SQLite (WAL, synchronous=FULL) right after
//! transcription, before any rewrite or paste is attempted. A crash, a failed paste or a hung LLM
//! can therefore never lose what was said.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::mode::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// ASR text saved; rewrite/paste not finished (crash recovery picks these up).
    Transcribed,
    Inserted,
    /// Paste was not possible (elevated window, focus lost); text left on the clipboard.
    ClipboardOnly,
    Failed,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Transcribed => "transcribed",
            Status::Inserted => "inserted",
            Status::ClipboardOnly => "clipboard_only",
            Status::Failed => "failed",
        }
    }
    fn parse(s: &str) -> Status {
        match s {
            "inserted" => Status::Inserted,
            "clipboard_only" => Status::ClipboardOnly,
            "failed" => Status::Failed,
            _ => Status::Transcribed,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Dictation {
    pub id: i64,
    pub created_ms: i64,
    pub app_exe: String,
    pub mode: Mode,
    pub raw_text: String,
    pub final_text: Option<String>,
    pub status: Status,
    pub error: Option<String>,
    pub audio_ms: i64,
    pub asr_ms: i64,
    pub llm_ms: Option<i64>,
    pub llm_used: bool,
    pub inject_ms: Option<i64>,
    pub total_ms: Option<i64>,
}

impl Dictation {
    /// The best text we have: rewritten if available, otherwise raw.
    pub fn best_text(&self) -> &str {
        self.final_text.as_deref().unwrap_or(&self.raw_text)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct LatencyStats {
    pub count: usize,
    pub p50_total_ms: Option<i64>,
    pub p95_total_ms: Option<i64>,
    pub p50_asr_ms: Option<i64>,
    pub p50_llm_ms: Option<i64>,
    pub words_total: usize,
}

pub struct Store {
    conn: Connection,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

const COLS: &str = "id, created_ms, app_exe, mode, raw_text, final_text, status, error, audio_ms, \
                    asr_ms, llm_ms, llm_used, inject_ms, total_ms";

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dictations (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                created_ms  INTEGER NOT NULL,
                app_exe     TEXT NOT NULL DEFAULT '',
                mode        TEXT NOT NULL,
                raw_text    TEXT NOT NULL,
                final_text  TEXT,
                status      TEXT NOT NULL,
                error       TEXT,
                audio_ms    INTEGER NOT NULL DEFAULT 0,
                asr_ms      INTEGER NOT NULL DEFAULT 0,
                llm_ms      INTEGER,
                llm_used    INTEGER NOT NULL DEFAULT 0,
                inject_ms   INTEGER,
                total_ms    INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_dictations_created ON dictations(created_ms DESC);",
        )?;
        Ok(Self { conn })
    }

    /// Commit the raw transcript. Call this before rewriting or pasting.
    pub fn insert_transcribed(
        &self,
        app_exe: &str,
        mode: Mode,
        raw: &str,
        audio_ms: i64,
        asr_ms: i64,
    ) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO dictations (created_ms, app_exe, mode, raw_text, status, audio_ms, asr_ms)
             VALUES (?1, ?2, ?3, ?4, 'transcribed', ?5, ?6)",
            params![now_ms(), app_exe, mode_str(mode), raw, audio_ms, asr_ms],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_final(
        &self,
        id: i64,
        final_text: &str,
        llm_used: bool,
        llm_ms: Option<i64>,
        note: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE dictations SET final_text = ?2, llm_used = ?3, llm_ms = ?4, error = ?5 WHERE id = ?1",
            params![id, final_text, llm_used, llm_ms, note],
        )?;
        Ok(())
    }

    pub fn set_status(
        &self,
        id: i64,
        status: Status,
        inject_ms: Option<i64>,
        total_ms: Option<i64>,
        error: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE dictations SET status = ?2, inject_ms = ?3, total_ms = ?4, error = COALESCE(?5, error) WHERE id = ?1",
            params![id, status.as_str(), inject_ms, total_ms, error],
        )?;
        Ok(())
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Dictation>> {
        self.conn.query_row(&format!("SELECT {COLS} FROM dictations WHERE id = ?1"), [id], row).optional()
    }

    /// Most recent dictation of any status: what "re-paste last" inserts.
    pub fn last(&self) -> rusqlite::Result<Option<Dictation>> {
        self.conn.query_row(&format!("SELECT {COLS} FROM dictations ORDER BY id DESC LIMIT 1"), [], row).optional()
    }

    /// Dictations that never reached the target app (crash, focus loss, UIPI).
    pub fn unfinished(&self, since_ms: i64) -> rusqlite::Result<Vec<Dictation>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS} FROM dictations WHERE status IN ('transcribed','clipboard_only','failed')
             AND created_ms >= ?1 ORDER BY id DESC"
        ))?;
        st.query_map([since_ms], row)?.collect()
    }

    /// Newest first; `query` matches raw or final text (case-insensitive).
    pub fn search(&self, query: &str, limit: usize, offset: usize) -> rusqlite::Result<Vec<Dictation>> {
        let like = format!("%{}%", query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS} FROM dictations
             WHERE ?1 = '' OR raw_text LIKE ?2 ESCAPE '\\' OR final_text LIKE ?2 ESCAPE '\\'
             ORDER BY id DESC LIMIT ?3 OFFSET ?4"
        ))?;
        st.query_map(params![query, like, limit as i64, offset as i64], row)?.collect()
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM dictations WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM dictations", [])?;
        Ok(())
    }

    pub fn prune(&self, retention_days: u32) -> rusqlite::Result<usize> {
        let cutoff = now_ms() - i64::from(retention_days) * 86_400_000;
        self.conn.execute("DELETE FROM dictations WHERE created_ms < ?1", [cutoff])
    }

    pub fn stats(&self, last_n: usize) -> rusqlite::Result<LatencyStats> {
        let rows = self.search("", last_n, 0)?;
        let pct = |mut v: Vec<i64>, p: f64| -> Option<i64> {
            if v.is_empty() {
                return None;
            }
            v.sort_unstable();
            let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
            Some(v[idx])
        };
        let ok: Vec<&Dictation> = rows.iter().filter(|d| d.status == Status::Inserted).collect();
        Ok(LatencyStats {
            count: rows.len(),
            p50_total_ms: pct(ok.iter().filter_map(|d| d.total_ms).collect(), 0.5),
            p95_total_ms: pct(ok.iter().filter_map(|d| d.total_ms).collect(), 0.95),
            p50_asr_ms: pct(ok.iter().map(|d| d.asr_ms).collect(), 0.5),
            p50_llm_ms: pct(ok.iter().filter_map(|d| d.llm_ms).collect(), 0.5),
            words_total: rows.iter().map(|d| crate::normalize::word_count(d.best_text())).sum(),
        })
    }
}

fn mode_str(m: Mode) -> String {
    serde_json::to_value(m).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Dictation> {
    let mode: String = r.get(3)?;
    let status: String = r.get(6)?;
    Ok(Dictation {
        id: r.get(0)?,
        created_ms: r.get(1)?,
        app_exe: r.get(2)?,
        mode: serde_json::from_value(serde_json::Value::String(mode)).unwrap_or(Mode::Clean),
        raw_text: r.get(4)?,
        final_text: r.get(5)?,
        status: Status::parse(&status),
        error: r.get(7)?,
        audio_ms: r.get(8)?,
        asr_ms: r.get(9)?,
        llm_ms: r.get(10)?,
        llm_used: r.get(11)?,
        inject_ms: r.get(12)?,
        total_ms: r.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_and_recovery() {
        let s = Store::open_in_memory().unwrap();
        let id = s.insert_transcribed("Code.exe", Mode::Technical, "um fix the bug", 1800, 210).unwrap();
        // Crash here: the raw text is already durable and shows up as unfinished.
        let pending = s.unfinished(0).unwrap();
        assert_eq!((pending.len(), pending[0].best_text()), (1, "um fix the bug"));

        s.set_final(id, "Fix the bug.", true, Some(430), None).unwrap();
        s.set_status(id, Status::Inserted, Some(25), Some(700), None).unwrap();
        let d = s.last().unwrap().unwrap();
        assert_eq!(
            (d.best_text(), d.status, d.mode, d.total_ms),
            ("Fix the bug.", Status::Inserted, Mode::Technical, Some(700))
        );
        assert!(s.unfinished(0).unwrap().is_empty());
    }

    #[test]
    fn search_escapes_like_wildcards() {
        let s = Store::open_in_memory().unwrap();
        s.insert_transcribed("", Mode::Clean, "100% done", 0, 0).unwrap();
        s.insert_transcribed("", Mode::Clean, "1000 done", 0, 0).unwrap();
        s.insert_transcribed("", Mode::Clean, "snake_case", 0, 0).unwrap();
        s.insert_transcribed("", Mode::Clean, "snakeXcase", 0, 0).unwrap();
        assert_eq!(s.search("0%", 10, 0).unwrap().len(), 1);
        assert_eq!(s.search("e_c", 10, 0).unwrap().len(), 1);
        assert_eq!(s.search("", 10, 0).unwrap().len(), 4);
        assert_eq!(s.search("DONE", 10, 0).unwrap().len(), 2, "case-insensitive");
    }

    #[test]
    fn stats_use_inserted_rows() {
        let s = Store::open_in_memory().unwrap();
        for (i, total) in [600, 700, 800, 2000].iter().enumerate() {
            let id = s.insert_transcribed("", Mode::Clean, "one two", 0, 100 + i as i64).unwrap();
            s.set_status(id, Status::Inserted, Some(10), Some(*total), None).unwrap();
        }
        let failed = s.insert_transcribed("", Mode::Clean, "x", 0, 0).unwrap();
        s.set_status(failed, Status::Failed, None, Some(99_999), Some("boom")).unwrap();
        let st = s.stats(100).unwrap();
        assert_eq!((st.count, st.p50_total_ms, st.p95_total_ms), (5, Some(800), Some(2000)));
    }

    #[test]
    fn durable_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("voxora.db");
        let id = Store::open(&p).unwrap().insert_transcribed("", Mode::Clean, "keep me", 0, 0).unwrap();
        assert_eq!(Store::open(&p).unwrap().get(id).unwrap().unwrap().raw_text, "keep me");
    }
}
