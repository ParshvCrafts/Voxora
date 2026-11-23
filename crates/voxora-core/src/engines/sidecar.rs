//! Supervises an inference server (whisper-server, llama-server) as a child process bound to
//! 127.0.0.1. Health-gated start, adoption of an already-running healthy server, log capture,
//! restart on demand.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Spec {
    pub name: &'static str,
    pub exe: PathBuf,
    pub args: Vec<String>,
    pub port: u16,
    /// GET path that returns 200 only once the model is loaded.
    pub health_path: &'static str,
    pub log_path: PathBuf,
}

type SpawnHook = Box<dyn Fn(&Child) + Send + Sync>;

pub struct Sidecar {
    pub spec: Spec,
    child: Mutex<Option<Child>>,
    http: reqwest::Client,
    on_spawn: Option<SpawnHook>,
}

impl Sidecar {
    /// `on_spawn` runs right after the process starts (the app uses it to put the child in a
    /// kill-on-close job object so a crash never leaves a server holding VRAM).
    pub fn new(spec: Spec, on_spawn: Option<SpawnHook>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .no_proxy()
            .build()
            .expect("static reqwest config");
        Self { spec, child: Mutex::new(None), http, on_spawn }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.spec.port)
    }

    pub async fn healthy(&self) -> bool {
        let url = format!("{}{}", self.url(), self.spec.health_path);
        matches!(self.http.get(url).send().await, Ok(r) if r.status().is_success())
    }

    fn child_exited(&self) -> Option<String> {
        let mut guard = self.child.lock().unwrap();
        let child = guard.as_mut()?;
        match child.try_wait() {
            Ok(Some(status)) => {
                *guard = None;
                Some(format!("exited with {status}"))
            }
            Ok(None) => None,
            Err(e) => Some(e.to_string()),
        }
    }

    /// Start (or adopt) the server and wait until it reports healthy.
    pub async fn ensure_running(&self, ready_timeout: Duration) -> Result<(), String> {
        if self.healthy().await {
            return Ok(());
        }
        let running = self.child.lock().unwrap().is_some() && self.child_exited().is_none();
        if !running {
            self.spawn()?;
        }
        let deadline = Instant::now() + ready_timeout;
        loop {
            if self.healthy().await {
                return Ok(());
            }
            if let Some(why) = self.child_exited() {
                return Err(format!("{} {why}. Log tail:\n{}", self.spec.name, self.log_tail(12)));
            }
            if Instant::now() > deadline {
                return Err(format!("{} not ready after {:?}", self.spec.name, ready_timeout));
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }

    fn spawn(&self) -> Result<(), String> {
        if !self.spec.exe.exists() {
            return Err(format!("{} binary missing: {}", self.spec.name, self.spec.exe.display()));
        }
        if let Some(dir) = self.spec.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let log = std::fs::File::create(&self.spec.log_path).map_err(|e| format!("log file: {e}"))?;
        let mut cmd = Command::new(&self.spec.exe);
        cmd.args(&self.spec.args).stdin(Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log);
        if let Some(dir) = self.spec.exe.parent() {
            cmd.current_dir(dir); // CUDA DLLs live next to the exe
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let child = cmd.spawn().map_err(|e| format!("failed to start {}: {e}", self.spec.name))?;
        if let Some(hook) = &self.on_spawn {
            hook(&child);
        }
        *self.child.lock().unwrap() = Some(child);
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    pub async fn restart(&self, ready_timeout: Duration) -> Result<(), String> {
        self.stop();
        self.ensure_running(ready_timeout).await
    }

    pub fn log_tail(&self, lines: usize) -> String {
        let text = std::fs::read_to_string(&self.spec.log_path).unwrap_or_default();
        let all: Vec<&str> = text.lines().collect();
        all[all.len().saturating_sub(lines)..].join("\n")
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        self.stop();
    }
}
