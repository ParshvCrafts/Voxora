//! Microphone capture. The stream opens only while recording (no always-on listening).
//!
//! The cpal callback downmixes to mono and appends native-rate samples to a buffer. A cheap
//! streaming 16 kHz copy feeds the earshot VAD and the RMS meter live; when recording stops,
//! the full buffer is resampled properly (FFT, anti-aliased) for Whisper.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

pub const RATE: u32 = 16_000;
const FRAME: usize = 256; // earshot frame: 16 ms @ 16 kHz
const VOICE_THRESHOLD: f32 = 0.5;

/// Live events from the capture thread.
#[derive(Debug, Clone)]
pub enum Event {
    /// ~30 Hz meter update, 0..1.
    Level(f32),
    /// Hands-free endpoint: speech happened, then `silence_ms` of quiet.
    Silence,
    /// Mic delivers digital zeros (muted / privacy switch / dead device).
    DeadMic,
    /// Stream failed mid-recording (device unplugged, exclusive-mode grab).
    StreamError(String),
}

pub struct Recording {
    /// 16 kHz mono.
    pub samples: Vec<f32>,
    /// Voiced duration according to the VAD.
    pub speech_ms: u32,
    pub duration_ms: u32,
}

enum Cmd {
    Start { device: Option<String>, silence_ms: u32, reply: mpsc::Sender<Result<(), String>> },
    Stop { reply: mpsc::Sender<Recording> },
    Cancel,
}

/// Handle to the audio thread (cpal streams are not `Send`, so they live on their own thread).
#[derive(Clone)]
pub struct Audio {
    tx: mpsc::Sender<Cmd>,
}

pub fn input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| devs.filter_map(|d| d.description().ok().map(|x| x.name().to_string())).collect())
        .unwrap_or_default()
}

struct Shared {
    native: Vec<f32>,
    native_rate: u32,
    vad_tail: Vec<f32>,
    resample_pos: f64,
    speech_frames: u32,
    any_speech: bool,
    quiet_since_speech_ms: u32,
    peak_abs: f32,
    level_acc: (f32, usize),
}

impl Audio {
    pub fn spawn(on_event: impl Fn(Event) + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        std::thread::Builder::new()
            .name("voxora-audio".into())
            .spawn(move || audio_thread(rx, on_event))
            .expect("spawn audio thread");
        Self { tx }
    }

    pub fn start(&self, device: Option<String>, silence_ms: u32) -> Result<(), String> {
        let (reply, rx) = mpsc::channel();
        self.tx.send(Cmd::Start { device, silence_ms, reply }).map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(3)).map_err(|_| "microphone did not start in 3 s".to_string())?
    }

    pub fn stop(&self) -> Option<Recording> {
        let (reply, rx) = mpsc::channel();
        self.tx.send(Cmd::Stop { reply }).ok()?;
        rx.recv_timeout(Duration::from_secs(5)).ok()
    }

    pub fn cancel(&self) {
        let _ = self.tx.send(Cmd::Cancel);
    }
}

fn audio_thread(rx: mpsc::Receiver<Cmd>, on_event: impl Fn(Event) + Send + Sync + 'static) {
    let on_event: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(on_event);
    let mut active: Option<(cpal::Stream, Arc<Mutex<Shared>>, Instant)> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start { device, silence_ms, reply } => {
                active = None;
                match open_stream(device.as_deref(), silence_ms, on_event.clone()) {
                    Ok((stream, shared)) => {
                        active = Some((stream, shared, Instant::now()));
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Cmd::Stop { reply } => {
                let Some((stream, shared, started)) = active.take() else {
                    let _ = reply.send(Recording { samples: vec![], speech_ms: 0, duration_ms: 0 });
                    continue;
                };
                drop(stream); // closes the device; Windows mic indicator turns off
                let s = std::mem::replace(&mut *shared.lock().unwrap(), Shared::new(0));
                let samples = resample_to_16k(&s.native, s.native_rate);
                let _ = reply.send(Recording {
                    speech_ms: s.speech_frames * 16,
                    duration_ms: started.elapsed().as_millis() as u32,
                    samples,
                });
            }
            Cmd::Cancel => active = None,
        }
    }
}

impl Shared {
    fn new(native_rate: u32) -> Self {
        Self {
            native: Vec::with_capacity(native_rate as usize * 30),
            native_rate,
            vad_tail: Vec::with_capacity(FRAME * 2),
            resample_pos: 0.0,
            speech_frames: 0,
            any_speech: false,
            quiet_since_speech_ms: 0,
            peak_abs: 0.0,
            level_acc: (0.0, 0),
        }
    }
}

fn find_device(name: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    if let Some(want) = name {
        let found =
            host.input_devices().map_err(|e| e.to_string())?.find(|d| d.description().is_ok_and(|x| x.name() == want));
        if let Some(d) = found {
            return Ok(d);
        }
        tracing::warn!("input device {want:?} not found; using the Windows default");
    }
    host.default_input_device().ok_or_else(|| "no microphone found".to_string())
}

fn open_stream(
    device: Option<&str>,
    silence_ms: u32,
    on_event: Arc<dyn Fn(Event) + Send + Sync>,
) -> Result<(cpal::Stream, Arc<Mutex<Shared>>), String> {
    let dev = find_device(device)?;
    let cfg = dev.default_input_config().map_err(|e| format!("microphone config: {e}"))?;
    let channels = cfg.channels() as usize;
    let rate = cfg.sample_rate();
    let shared = Arc::new(Mutex::new(Shared::new(rate)));
    let vad = Arc::new(Mutex::new(earshot::Detector::default()));
    let dead_reported = Arc::new(AtomicBool::new(false));
    let max_samples = rate as usize * 60 * 10; // 10 minute cap

    let push = {
        let shared = shared.clone();
        let on_event = on_event.clone();
        move |mono: &[f32]| {
            let mut s = shared.lock().unwrap();
            if s.native.len() >= max_samples {
                return;
            }
            s.native.extend_from_slice(mono);
            for x in mono {
                s.peak_abs = s.peak_abs.max(x.abs());
                s.level_acc.0 += x * x;
                s.level_acc.1 += 1;
            }
            // ~30 Hz level meter.
            if s.level_acc.1 >= (s.native_rate / 30) as usize {
                let rms = (s.level_acc.0 / s.level_acc.1 as f32).sqrt();
                s.level_acc = (0.0, 0);
                // dBFS meter: -50 dB (room noise) .. -5 dB (loud speech) mapped to 0..1.
                let db = 20.0 * rms.max(1e-6).log10();
                on_event(Event::Level(((db + 50.0) / 45.0).clamp(0.0, 1.0)));
            }
            // Streaming linear resample to 16 kHz for the VAD only.
            let step = s.native_rate as f64 / RATE as f64;
            let mut pos = s.resample_pos;
            while pos + 1.0 < mono.len() as f64 {
                let i = pos.floor() as usize;
                let f = (pos - i as f64) as f32;
                let v = mono[i] * (1.0 - f) + mono[i + 1] * f;
                s.vad_tail.push(v.clamp(-1.0, 1.0));
                pos += step;
            }
            s.resample_pos = pos - mono.len() as f64;
            while s.vad_tail.len() >= FRAME {
                let frame: Vec<f32> = s.vad_tail.drain(..FRAME).collect();
                let score = vad.lock().unwrap().predict_f32(&frame);
                if score >= VOICE_THRESHOLD {
                    s.speech_frames += 1;
                    s.any_speech = true;
                    s.quiet_since_speech_ms = 0;
                } else if s.any_speech {
                    s.quiet_since_speech_ms += 16;
                    if s.quiet_since_speech_ms >= silence_ms && s.quiet_since_speech_ms < silence_ms + 16 {
                        on_event(Event::Silence);
                    }
                }
            }
            // Digital silence for 1.5 s = muted or dead input.
            let secs = s.native.len() as f32 / s.native_rate as f32;
            if secs > 1.5 && s.peak_abs == 0.0 && !dead_reported.swap(true, Ordering::Relaxed) {
                on_event(Event::DeadMic);
            }
        }
    };

    let err_cb = {
        let on_event = on_event.clone();
        move |e: cpal::Error| {
            if ends_stream(e.kind()) {
                on_event(Event::StreamError(e.to_string()));
            } else {
                tracing::debug!("audio stream hiccup, recording continues: {e}");
            }
        }
    };
    let stream_cfg: cpal::StreamConfig = cfg.config();
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => dev.build_input_stream(
            stream_cfg,
            move |data: &[f32], _| push(&downmix(data, channels, |x| x)),
            err_cb,
            None,
        ),
        cpal::SampleFormat::I16 => dev.build_input_stream(
            stream_cfg,
            move |data: &[i16], _| push(&downmix(data, channels, |x| x as f32 / 32768.0)),
            err_cb,
            None,
        ),
        cpal::SampleFormat::I32 => dev.build_input_stream(
            stream_cfg,
            move |data: &[i32], _| push(&downmix(data, channels, |x| x as f32 / 2_147_483_648.0)),
            err_cb,
            None,
        ),
        other => return Err(format!("unsupported microphone sample format {other:?}")),
    }
    .map_err(|e| format!("cannot open microphone: {e}"))?;
    stream.play().map_err(|e| format!("cannot start microphone: {e}"))?;
    Ok((stream, shared))
}

fn downmix<T: Copy>(data: &[T], channels: usize, conv: impl Fn(T) -> f32) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|x| conv(*x)).collect();
    }
    data.chunks_exact(channels).map(|f| f.iter().map(|x| conv(*x)).sum::<f32>() / channels as f32).collect()
}

/// High-quality FFT resample of a whole clip to 16 kHz mono.
pub fn resample_to_16k(input: &[f32], rate: u32) -> Vec<f32> {
    if rate == RATE || input.is_empty() {
        return input.to_vec();
    }
    let mut r = match Fft::<f32>::new(rate as usize, RATE as usize, 1024, 1, FixedSync::Input) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("resampler init failed ({e}); falling back to linear");
            return linear_resample(input, rate);
        }
    };
    let adapter = InterleavedSlice::new(input, 1, input.len()).expect("mono slice adapter");
    match r.process_all(&adapter, input.len(), None) {
        Ok(out) => out.take_data(),
        Err(e) => {
            tracing::error!("resample failed ({e}); falling back to linear");
            linear_resample(input, rate)
        }
    }
}

fn linear_resample(input: &[f32], rate: u32) -> Vec<f32> {
    let step = rate as f64 / RATE as f64;
    let n = (input.len() as f64 / step) as usize;
    (0..n)
        .map(|k| {
            let p = k as f64 * step;
            let i = p as usize;
            let f = (p - i as f64) as f32;
            input[i] * (1.0 - f) + input.get(i + 1).copied().unwrap_or(input[i]) * f
        })
        .collect()
}

/// Whether a cpal stream error means the recording is over. Laptop mics often report one buffer
/// overrun as the stream starts; treating that as "disconnected" cancelled every dictation on
/// the built-in Realtek mic (v0.2.0 bug). Glitches and automatic reroutes keep the stream alive.
fn ends_stream(kind: cpal::ErrorKind) -> bool {
    !matches!(kind, cpal::ErrorKind::Xrun | cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_glitches_do_not_cancel_a_dictation() {
        assert!(!ends_stream(cpal::ErrorKind::Xrun), "an overrun at stream start must not cancel");
        assert!(!ends_stream(cpal::ErrorKind::DeviceChanged), "a rerouted stream keeps running");
        assert!(ends_stream(cpal::ErrorKind::DeviceNotAvailable), "an unplugged mic does end it");
        assert!(ends_stream(cpal::ErrorKind::StreamInvalidated));
    }

    fn sine(rate: u32, secs: f32, hz: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize)
            .map(|i| (i as f32 * hz * std::f32::consts::TAU / rate as f32).sin())
            .collect()
    }

    #[test]
    fn resample_preserves_duration_and_tone() {
        let out = resample_to_16k(&sine(48_000, 1.0, 440.0), 48_000);
        assert!((out.len() as i64 - 16_000).abs() < 64, "len {}", out.len());
        // Zero crossings of a 440 Hz tone over 1 s ≈ 880.
        let zc = out.windows(2).filter(|w| w[0].signum() != w[1].signum()).count();
        assert!((860..=900).contains(&zc), "zero crossings {zc}");
    }

    #[test]
    fn resample_44k1_and_passthrough() {
        let out = resample_to_16k(&sine(44_100, 0.5, 300.0), 44_100);
        assert!((out.len() as i64 - 8_000).abs() < 64);
        let same = vec![0.1; 100];
        assert_eq!(resample_to_16k(&same, 16_000), same);
    }

    #[test]
    fn downmix_averages_channels() {
        assert_eq!(downmix(&[1.0f32, 0.0, 0.5, 0.5], 2, |x| x), vec![0.5, 0.5]);
        assert_eq!(downmix(&[i16::MIN], 1, |x| x as f32 / 32768.0), vec![-1.0]);
    }
}
