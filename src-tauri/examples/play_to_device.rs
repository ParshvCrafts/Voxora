//! Test helper: play a 16-bit WAV into a named output device (e.g. "CABLE Input" of VB-Audio
//! Virtual Cable) so Voxora can "hear" it through a real capture device.
//!
//! cargo run -p voxora --example play_to_device -- <file.wav> "<device substring>"
//! cargo run -p voxora --example play_to_device -- --list

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let host = cpal::default_host();
    let name = |d: &cpal::Device| d.description().map(|x| x.name().to_string()).unwrap_or_default();
    if args.first().map(String::as_str) == Some("--list") {
        for d in host.output_devices().unwrap() {
            println!("output: {}", name(&d));
        }
        for d in host.input_devices().unwrap() {
            println!("input:  {}", name(&d));
        }
        return;
    }
    let (path, want) = (&args[0], args.get(1).map(String::as_str).unwrap_or("CABLE Input"));
    let (samples, rate) = voxora_core::engines::whisper::decode_wav(&std::fs::read(path).unwrap()).unwrap();
    let dev = host.output_devices().unwrap().find(|d| name(d).contains(want)).expect("output device not found");
    let cfg = dev.default_output_config().unwrap();
    let (out_rate, ch) = (cfg.sample_rate(), cfg.channels() as usize);
    // Linear resample is fine for a test signal.
    let step = rate as f64 / out_rate as f64;
    let n = (samples.len() as f64 / step) as usize;
    let res: Vec<f32> = (0..n).map(|k| samples[((k as f64 * step) as usize).min(samples.len() - 1)]).collect();
    let pos = Arc::new(Mutex::new(0usize));
    let done = Arc::new(Mutex::new(false));
    let (p2, d2, data) = (pos.clone(), done.clone(), res.clone());
    let stream = dev
        .build_output_stream(
            cfg.config(),
            move |out: &mut [f32], _| {
                let mut p = p2.lock().unwrap();
                for frame in out.chunks_mut(ch) {
                    let v = data.get(*p).copied().unwrap_or(0.0);
                    frame.fill(v);
                    *p += 1;
                }
                if *p >= data.len() {
                    *d2.lock().unwrap() = true;
                }
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .expect("output stream (is the device f32?)");
    stream.play().unwrap();
    while !*done.lock().unwrap() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    println!("played {} s into {}", res.len() as f32 / out_rate as f32, name(&dev));
}
