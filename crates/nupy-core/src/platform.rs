//! 플랫폼별 바이너리 이름과 배포처.
//!
//! - macOS: Apple Silicon(arm64)만 지원한다.
//! - Windows: x64, arm64.

use crate::{Error, Result};

/// bin 폴더 안에 저장되는 yt-dlp 실행 파일 이름.
#[cfg(windows)]
pub const YTDLP_BIN: &str = "yt-dlp.exe";
#[cfg(target_os = "macos")]
pub const YTDLP_BIN: &str = "yt-dlp_macos";
#[cfg(not(any(windows, target_os = "macos")))]
pub const YTDLP_BIN: &str = "yt-dlp";

#[cfg(windows)]
pub const FFMPEG_BIN: &str = "ffmpeg.exe";
#[cfg(not(windows))]
pub const FFMPEG_BIN: &str = "ffmpeg";

/// macOS onedir 빌드가 함께 쓰는 런타임 폴더.
pub const YTDLP_INTERNAL_DIR: &str = "_internal";

pub const YTDLP_REPO: &str = "yt-dlp/yt-dlp";
pub const YTDLP_SUMS_ASSET: &str = "SHA2-256SUMS";

/// yt-dlp 릴리스에서 받을 파일.
#[derive(Debug, Clone, Copy)]
pub enum YtdlpAsset {
    /// 실행 파일 + `_internal/` 이 들어 있는 zip (macOS onedir 빌드, 실행 속도가 빠르다)
    OnedirZip(&'static str),
    /// 단일 실행 파일
    Executable(&'static str),
}

impl YtdlpAsset {
    pub fn name(self) -> &'static str {
        match self {
            YtdlpAsset::OnedirZip(n) | YtdlpAsset::Executable(n) => n,
        }
    }
}

pub fn ytdlp_asset() -> Result<YtdlpAsset> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(YtdlpAsset::OnedirZip("yt-dlp_macos.zip")),
        ("windows", "x86_64") => Ok(YtdlpAsset::Executable("yt-dlp.exe")),
        ("windows", "aarch64") => Ok(YtdlpAsset::Executable("yt-dlp_arm64.exe")),
        _ => Err(Error::UnsupportedPlatform(target_label())),
    }
}

/// ffmpeg 배포처.
#[derive(Debug, Clone, Copy)]
pub struct FfmpegSource {
    /// 앞에서부터 차례로 시도할 zip URL.
    pub urls: &'static [&'static str],
    /// zip 안에서 ffmpeg 실행 파일을 찾을 때 쓸 경로 접미사.
    pub entry_suffix: &'static str,
    pub checksum: ChecksumSource,
}

#[derive(Debug, Clone, Copy)]
pub enum ChecksumSource {
    /// 최종(리다이렉트 후) URL 뒤에 붙여 받는 `.sha256` 파일
    Sibling(&'static str),
    /// 여러 파일의 해시가 적힌 목록 파일
    List(&'static str),
}

pub fn ffmpeg_source() -> Result<FfmpegSource> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        // martin-riedl.de: Apple Silicon 네이티브 정적 빌드. release가 없을 때는 snapshot을 쓴다.
        ("macos", "aarch64") => Ok(FfmpegSource {
            urls: &[
                "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip",
                "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/snapshot/ffmpeg.zip",
            ],
            entry_suffix: "ffmpeg",
            checksum: ChecksumSource::Sibling(".sha256"),
        }),
        ("windows", "x86_64") => Ok(FfmpegSource {
            urls: &["https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"],
            entry_suffix: "bin/ffmpeg.exe",
            checksum: ChecksumSource::List(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256",
            ),
        }),
        ("windows", "aarch64") => Ok(FfmpegSource {
            urls: &["https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-winarm64-gpl.zip"],
            entry_suffix: "bin/ffmpeg.exe",
            checksum: ChecksumSource::List(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256",
            ),
        }),
        _ => Err(Error::UnsupportedPlatform(target_label())),
    }
}

fn target_label() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64") => "Intel Mac",
        ("macos", _) => "macOS",
        ("windows", _) => "Windows",
        ("linux", _) => "Linux",
        _ => "unknown",
    }
}
