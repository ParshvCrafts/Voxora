//! Resumable, SHA-256-verified downloads of pinned models and runtimes.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Single file placed in the models directory.
    Model,
    /// Archive (zip or tar.gz) extracted into `runtime/<id>/`.
    Runtime,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Asset {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    pub file: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

/// Pinned, verified 2026-11-23 (see docs/DECISIONS.md D2, D6).
pub const CATALOG: &[Asset] = &[
    Asset {
        id: "whisper-cuda",
        label: "whisper.cpp b5907 server (CUDA 12.4)",
        kind: Kind::Runtime,
        file: "whisper-bin-win-cuda-12.4.0-x64.zip",
        url: "https://github.com/ggml-org/whisper.cpp/releases/download/b5907/whisper-bin-win-cuda-12.4.0-x64.zip",
        sha256: "34f872ce724ca1ff0a4afca481853e5ee74250f8cb22153e499f953562846039",
        bytes: 735_571_214,
    },
    Asset {
        id: "whisper-cpu",
        label: "whisper.cpp b5907 server (CPU)",
        kind: Kind::Runtime,
        file: "whisper-bin-x64.zip",
        url: "https://github.com/ggml-org/whisper.cpp/releases/download/b5907/whisper-bin-x64.zip",
        sha256: "ecca07b277d943e9b9f4712856a144b857271a6b333104e511e1988e80db582b",
        bytes: 9_168_428,
    },
    Asset {
        id: "llama-cuda",
        label: "llama.cpp b12649 server (CUDA 12.4)",
        kind: Kind::Runtime,
        file: "llama-b12649-bin-win-cuda-12.4-x64.zip",
        url: "https://github.com/ggml-org/llama.cpp/releases/download/b12649/llama-b12649-bin-win-cuda-12.4-x64.zip",
        sha256: "f512ff1fcabf545a073b29ecb848611bee02fe564d22709f0e50b4972e18ee46",
        bytes: 358_031_782,
    },
    Asset {
        id: "llama-cpu",
        label: "llama.cpp b12649 server (CPU)",
        kind: Kind::Runtime,
        file: "llama-b12649-bin-win-cpu-x64.zip",
        url: "https://github.com/ggml-org/llama.cpp/releases/download/b12649/llama-b12649-bin-win-cpu-x64.zip",
        sha256: "0e2ff97a5ef40de3e4ca910231676c044e0813c54727b2012022a2d2401e1fa9",
        bytes: 25_096_466,
    },
    Asset {
        // Metal build for Apple silicon. Dylibs are versioned symlinks: extract with symlinks.
        id: "llama-macos",
        label: "llama.cpp b12649 server (Metal)",
        kind: Kind::Runtime,
        file: "llama-b12649-bin-macos-arm64.tar.gz",
        url: "https://github.com/ggml-org/llama.cpp/releases/download/b12649/llama-b12649-bin-macos-arm64.tar.gz",
        sha256: "04db8281b11189b91b159c7890688b7dc3bc62a95fe81ec5c8078067be5bb4ad",
        bytes: 16_643_876,
    },
    Asset {
        id: "whisper-large-v3-turbo-q8",
        label: "Whisper large-v3-turbo (Q8, 874 MB)",
        kind: Kind::Model,
        file: "ggml-large-v3-turbo-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin",
        sha256: "317eb69c11673c9de1e1f0d459b253999804ec71ac4c23c17ecf5fbe24e259a1",
        bytes: 874_188_075,
    },
    Asset {
        id: "silero-vad",
        label: "Silero VAD v6.2",
        kind: Kind::Model,
        file: "ggml-silero-v6.2.0.bin",
        url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin",
        sha256: "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987",
        bytes: 885_098,
    },
    Asset {
        id: "qwen3.5-4b",
        label: "Qwen3.5 4B Instruct (Q4_K_M, 2.7 GB)",
        kind: Kind::Model,
        file: "Qwen3.5-4B-Q4_K_M.gguf",
        url: "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf",
        sha256: "00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4",
        bytes: 2_740_937_888,
    },
    Asset {
        id: "qwen3.5-2b",
        label: "Qwen3.5 2B Instruct (Q4_K_M, 1.3 GB)",
        kind: Kind::Model,
        file: "Qwen3.5-2B-Q4_K_M.gguf",
        url: "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
        sha256: "aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223",
        bytes: 1_280_835_840,
    },
];

pub fn asset(id: &str) -> Option<&'static Asset> {
    CATALOG.iter().find(|a| a.id == id)
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Download `url` to `dest`, resuming a `.part` file if present, verifying SHA-256 before the
/// final rename. `progress(done, total)` is called as bytes arrive.
pub async fn download(url: &str, dest: &Path, sha256: &str, progress: impl Fn(u64, u64)) -> Result<PathBuf, String> {
    if dest.exists() && sha256_file(dest).map_err(|e| e.to_string())? == sha256 {
        return Ok(dest.to_path_buf());
    }
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let part = dest.with_extension(format!("{}.part", dest.extension().and_then(|e| e.to_str()).unwrap_or("")));
    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .user_agent(concat!("Voxora/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = String::new();
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
        }
        match fetch_into(&http, url, &part, &progress).await {
            Ok(()) => {
                let got = sha256_file(&part).map_err(|e| e.to_string())?;
                if got != sha256 {
                    let _ = std::fs::remove_file(&part);
                    return Err(format!("checksum mismatch for {url}: expected {sha256}, got {got}"));
                }
                std::fs::rename(&part, dest).map_err(|e| e.to_string())?;
                return Ok(dest.to_path_buf());
            }
            Err(e) => last_err = e,
        }
    }
    Err(format!("download failed after retries: {last_err}"))
}

async fn fetch_into(
    http: &reqwest::Client,
    url: &str,
    part: &Path,
    progress: &impl Fn(u64, u64),
) -> Result<(), String> {
    let have = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut req = http.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        return Ok(()); // already complete; checksum decides
    }
    if !status.is_success() {
        return Err(format!("HTTP {status} for {url}"));
    }
    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    let start = if resumed { have } else { 0 };
    let total = start + resp.content_length().unwrap_or(0);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(part)
        .map_err(|e| e.to_string())?;
    let mut done = start;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(|e| e.to_string())
}

/// Extract a runtime archive (`.zip` or `.tar.gz`) into `dir`, flattening a single top-level
/// folder if present.
pub fn extract(archive: &Path, dir: &Path) -> Result<(), String> {
    if archive.to_string_lossy().ends_with(".tar.gz") {
        extract_tar_gz(archive, dir)
    } else {
        extract_zip(archive, dir)
    }
}

/// tar.gz keeps Unix modes and symlinks (llama.cpp's macOS dylibs depend on both).
fn extract_tar_gz(path: &Path, dir: &Path) -> Result<(), String> {
    let open = || -> Result<tar::Archive<flate2::read::GzDecoder<std::fs::File>>, String> {
        let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        Ok(tar::Archive::new(flate2::read::GzDecoder::new(f)))
    };
    let names: Vec<String> = open()?
        .entries()
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().and_then(|e| e.path().ok().map(|p| p.to_string_lossy().replace('\\', "/"))))
        .collect();
    let prefix = common_root(&names);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut a = open()?;
    a.set_preserve_permissions(true);
    for e in a.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        let p = e.path().map_err(|e| e.to_string())?.into_owned();
        if p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("unsafe path in archive: {}", p.display()));
        }
        let rel = p.strip_prefix(&prefix).unwrap_or(&p).to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dir.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        e.unpack(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    Ok(())
}

/// Extract a runtime zip into `dir`, flattening a single top-level folder if present.
fn extract_zip(zip_path: &Path, dir: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let names: Vec<String> =
        (0..z.len()).filter_map(|i| z.by_index(i).ok().and_then(|e| e.name().ok().map(|n| n.to_string()))).collect();
    let prefix = common_root(&names);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = e.enclosed_name() else { continue }; // rejects ../ path traversal
        let rel = rel.strip_prefix(&prefix).unwrap_or(&rel).to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dir.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut e, &mut w).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn common_root(names: &[String]) -> PathBuf {
    let first = names.iter().find_map(|n| n.split('/').next().filter(|s| !s.is_empty()));
    match first {
        Some(root) if names.iter().all(|n| n.starts_with(&format!("{root}/"))) => PathBuf::from(root),
        _ => PathBuf::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_well_formed() {
        for a in CATALOG {
            assert_eq!(a.sha256.len(), 64, "{}", a.id);
            assert!(a.url.starts_with("https://") && a.url.ends_with(a.file), "{}", a.id);
        }
        let mut ids: Vec<_> = CATALOG.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn common_root_detection() {
        assert_eq!(common_root(&["build/a.exe".into(), "build/b.dll".into()]), PathBuf::from("build"));
        assert_eq!(common_root(&["a.exe".into(), "b.dll".into()]), PathBuf::new());
        assert_eq!(common_root(&["x/a".into(), "y/b".into()]), PathBuf::new());
    }

    #[test]
    fn zip_extract_flattens_and_blocks_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("t.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            w.start_file("Release/server.exe", o).unwrap();
            w.write_all(b"exe").unwrap();
            w.start_file("Release/../../evil.txt", o).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        let out = dir.path().join("out");
        extract_zip(&zp, &out).unwrap();
        assert_eq!(std::fs::read(out.join("server.exe")).unwrap(), b"exe");
        assert!(!dir.path().join("evil.txt").exists());
    }

    fn tar_gz(path: &Path, entries: &[(&str, &[u8])]) {
        let gz = flate2::write::GzEncoder::new(std::fs::File::create(path).unwrap(), flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (name, data) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o755);
            // set_path refuses "..", so write the raw name to build a hostile archive too.
            h.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name.as_bytes());
            h.set_cksum();
            b.append(&h, *data).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn tar_gz_extract_flattens_root_and_rejects_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("ok.tar.gz");
        tar_gz(&ok, &[("llama-b1/llama-server", b"bin"), ("llama-b1/lib/libggml.dylib", b"lib")]);
        let out = dir.path().join("out");
        extract(&ok, &out).unwrap();
        assert_eq!(std::fs::read(out.join("llama-server")).unwrap(), b"bin");
        assert_eq!(std::fs::read(out.join("lib/libggml.dylib")).unwrap(), b"lib");

        let bad = dir.path().join("bad.tar.gz");
        tar_gz(&bad, &[("x/../../evil", b"x")]);
        assert!(extract(&bad, &dir.path().join("out2")).is_err());
        assert!(!dir.path().join("evil").exists());
    }
}
