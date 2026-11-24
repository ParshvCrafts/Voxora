//! OpenAI-compatible chat client: works with the managed llama-server, Ollama (`/v1`), LM Studio.

use std::time::Duration;

use serde_json::{Value, json};

use crate::pipeline::Rewriter;
use crate::prompts;

#[derive(Clone)]
pub struct OpenAiCompat {
    http: reqwest::Client,
    /// Base URL ending in `/v1`.
    pub base_url: String,
    pub model: String,
}

impl OpenAiCompat {
    pub fn new(base_url: &str, model: &str) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .no_proxy() // localhost traffic must never go through a system proxy
            .build()
            .expect("static reqwest config");
        Self { http, base_url: base_url.trim_end_matches('/').to_string(), model: model.to_string() }
    }

    /// Send a 1-token request so the model is loaded and the prompt prefix cache is warm.
    pub async fn warm_up(&self) -> Result<(), String> {
        self.complete(prompts::messages(crate::mode::Mode::Clean, "warm up", "", &[]), 4).await.map(|_| ())
    }
}

impl Rewriter for OpenAiCompat {
    async fn complete(&self, messages: Vec<Value>, max_tokens: u32) -> Result<String, String> {
        let body = json!({
            "model": self.model,
            "messages": messages,
            // Greedy: a rewrite should be reproducible, not creative.
            "temperature": 0.0,
            "max_tokens": max_tokens,
            "stream": false,
            "response_format": prompts::response_format(),
            // llama-server: keep the shared system prompt + examples in the KV cache.
            "cache_prompt": true,
            // Qwen3.x and other hybrid-reasoning models: answer directly, no <think> block.
            "chat_template_kwargs": { "enable_thinking": false },
            "reasoning_effort": "none",
        });
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let v: Value = resp.json().await.map_err(|e| format!("bad response ({status}): {e}"))?;
        if !status.is_success() {
            let msg = v["error"]["message"].as_str().unwrap_or("unknown error");
            return Err(format!("HTTP {status}: {msg}"));
        }
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "reply had no message content".into())
    }
}
