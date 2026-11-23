//! Client for whisper.cpp's `whisper-server` (`POST /inference`, multipart WAV upload).

use std::time::Duration;

use serde_json::Value;

#[derive(Clone)]
pub struct WhisperClient {
    http: reqwest::Client,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
}

impl WhisperClient {
    pub fn new(port: u16) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(120))
            .no_proxy()
            .build()
            .expect("static reqwest config");
        Self { http, base_url: format!("http://127.0.0.1:{port}") }
    }

    /// `samples`: 16 kHz mono f32. `prompt` biases spelling toward the user's vocabulary.
    pub async fn transcribe(&self, samples: &[f32], language: &str, prompt: &str) -> Result<Transcript, String> {
        let wav = encode_wav(samples, 16_000);
        let file = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", file)
            .text("response_format", "json")
            .text("temperature", "0.0")
            .text("temperature_inc", "0.2")
            .text("language", language.to_string());
        if !prompt.is_empty() {
            form = form.text("prompt", prompt.to_string());
        }
        let resp = self
            .http
            .post(format!("{}/inference", self.base_url))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("whisper-server unreachable: {e}"))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("whisper-server HTTP {status}: {}", body.chars().take(200).collect::<String>()));
        }
        let v: Value = serde_json::from_str(&body).map_err(|e| format!("bad whisper JSON: {e}"))?;
        if let Some(err) = v["error"].as_str() {
            return Err(format!("whisper-server: {err}"));
        }
        Ok(Transcript { text: v["text"].as_str().unwrap_or("").trim().to_string() })
    }
}

/// Whisper's initial prompt: a natural sentence using the user's terms works better than a
/// bare list, and must stay well under the 224-token prompt window.
pub fn vocabulary_prompt(terms: &[String]) -> String {
    if terms.is_empty() {
        return String::new();
    }
    let mut s = String::from("Glossary: ");
    for (i, t) in terms.iter().enumerate() {
        if s.len() + t.len() > 600 {
            break;
        }
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(t);
    }
    s.push('.');
    s
}

/// 16-bit PCM mono WAV.
pub fn encode_wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    v.extend_from_slice(&2u16.to_le_bytes()); // block align
    v.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    v
}

/// Decode a 16-bit PCM WAV (any rate, mono or stereo) to mono f32 at its native rate.
pub fn decode_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let (mut pos, mut rate, mut channels, mut bits) = (12, 0u32, 0u16, 0u16);
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = &bytes[pos + 8..(pos + 8 + len).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            if bits != 16 || channels == 0 {
                return Err(format!("unsupported WAV: {bits}-bit, {channels} channels"));
            }
            let frames = body.chunks_exact(2 * channels as usize);
            let mono = frames
                .map(|f| {
                    let sum: f32 =
                        f.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).sum();
                    sum / channels as f32
                })
                .collect();
            return Ok((mono, rate));
        }
        pos += 8 + len + (len & 1);
    }
    Err("WAV has no data chunk".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip() {
        let s: Vec<f32> = (0..1600).map(|i| (i as f32 / 10.0).sin() * 0.5).collect();
        let (back, rate) = decode_wav(&encode_wav(&s, 16_000)).unwrap();
        assert_eq!((rate, back.len()), (16_000, s.len()));
        assert!(s.iter().zip(&back).all(|(a, b)| (a - b).abs() < 1e-3));
    }

    #[test]
    fn vocabulary_prompt_is_bounded() {
        assert_eq!(vocabulary_prompt(&[]), "");
        assert_eq!(vocabulary_prompt(&["PyTorch".into(), "FAISS".into()]), "Glossary: PyTorch, FAISS.");
        let many: Vec<String> = (0..500).map(|i| format!("Term{i}")).collect();
        assert!(vocabulary_prompt(&many).len() < 620);
    }
}
