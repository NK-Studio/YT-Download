//! yt-dlp / ffmpeg 설치와 업데이트.
//!
//! 바이너리는 사용자 데이터 폴더(`NKStudio/YTDownloader/bin`)에 둔다. 관리자 권한 없이
//! 교체할 수 있고, 기존 Swift(macOS) / WinUI(Windows) 버전이 쓰던 위치와 같아서
//! 이미 받아 둔 바이너리를 그대로 이어 쓴다.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::platform::{self, ChecksumSource, YtdlpAsset};
use crate::process::command;
use crate::{Error, Result};

const USER_AGENT: &str = concat!("NupyYTDownloader/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub bin_dir: PathBuf,
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
}

impl ToolPaths {
    pub fn new(bin_dir: impl Into<PathBuf>) -> Self {
        let bin_dir = bin_dir.into();
        Self {
            ytdlp: bin_dir.join(platform::YTDLP_BIN),
            ffmpeg: bin_dir.join(platform::FFMPEG_BIN),
            bin_dir,
        }
    }

    /// macOS: `~/Library/Application Support/NKStudio/YTDownloader/bin`
    /// Windows: `%APPDATA%\NKStudio\YTDownloader\bin`
    pub fn default_location() -> Result<Self> {
        let base = dirs::data_dir().ok_or_else(|| Error::Other("사용자 데이터 폴더를 찾지 못했습니다".into()))?;
        Ok(Self::new(base.join("NKStudio").join("YTDownloader").join("bin")))
    }

    pub fn ytdlp_installed(&self) -> bool {
        self.ytdlp.is_file()
    }

    pub fn ffmpeg_installed(&self) -> bool {
        self.ffmpeg.is_file()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub ytdlp_installed: bool,
    pub ytdlp_version: Option<String>,
    pub latest_ytdlp_version: Option<String>,
    pub ffmpeg_installed: bool,
    pub update_available: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Ytdlp,
    Ffmpeg,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallStage {
    Downloading,
    Verifying,
    Installing,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub tool: Tool,
    pub stage: InstallStage,
    /// 내려받기 진행률(0.0~1.0). 크기를 모르면 `None`.
    pub fraction: Option<f64>,
}

pub type ProgressFn<'a> = &'a (dyn Fn(InstallProgress) + Send + Sync);

pub struct ToolManager {
    paths: ToolPaths,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

impl ToolManager {
    pub fn new(paths: ToolPaths) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { paths, client })
    }

    pub fn paths(&self) -> &ToolPaths {
        &self.paths
    }

    /// 설치된 yt-dlp의 `--version` 출력.
    pub async fn local_ytdlp_version(&self) -> Option<String> {
        if !self.paths.ytdlp_installed() {
            return None;
        }
        let out = command(&self.paths.ytdlp).arg("--version").output().await.ok()?;
        if !out.status.success() {
            return None;
        }
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!v.is_empty()).then_some(v)
    }

    /// GitHub의 최신 안정 릴리스 태그 (예: `2026.10.26`).
    pub async fn latest_ytdlp_version(&self) -> Result<String> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", platform::YTDLP_REPO);
        let release: GithubRelease = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(release.tag_name.trim_start_matches('v').to_string())
    }

    /// 로컬/최신 버전을 모두 확인한 현재 상태. 최신 버전 조회 실패는 무시한다.
    pub async fn status(&self, check_latest: bool) -> ToolStatus {
        let ytdlp_version = self.local_ytdlp_version().await;
        let latest = if check_latest { self.latest_ytdlp_version().await.ok() } else { None };
        let update_available = matches!((&latest, &ytdlp_version), (Some(l), Some(c)) if is_newer(l, c));
        ToolStatus {
            ytdlp_installed: self.paths.ytdlp_installed(),
            ytdlp_version,
            latest_ytdlp_version: latest,
            ffmpeg_installed: self.paths.ffmpeg_installed(),
            update_available,
        }
    }

    /// yt-dlp 최신 릴리스를 내려받아 SHA-256을 검증한 뒤 교체한다. 설치된 버전을 반환한다.
    ///
    /// 호출하는 쪽에서 yt-dlp 프로세스가 돌고 있지 않음을 보장해야 한다
    /// (Windows는 실행 중인 exe를 덮어쓸 수 없다).
    pub async fn install_ytdlp(&self, tag: Option<&str>, on: ProgressFn<'_>) -> Result<String> {
        let asset = platform::ytdlp_asset()?;
        tokio::fs::create_dir_all(&self.paths.bin_dir).await?;

        // 같은 릴리스의 파일과 체크섬을 받도록 태그를 고정한다.
        // (API 호출이 막혀 태그를 모르면 latest 리다이렉트를 쓴다)
        let base = match tag {
            Some(t) => format!("https://github.com/{}/releases/download/{t}", platform::YTDLP_REPO),
            None => format!("https://github.com/{}/releases/latest/download", platform::YTDLP_REPO),
        };
        let asset_url = format!("{base}/{}", asset.name());
        let sums_url = format!("{base}/{}", platform::YTDLP_SUMS_ASSET);

        let staging = self.paths.bin_dir.join(".staging-ytdlp");
        remove_path(&staging).await?;
        tokio::fs::create_dir_all(&staging).await?;

        let result = async {
            let download_path = staging.join(asset.name());
            let report = |stage, fraction| on(InstallProgress { tool: Tool::Ytdlp, stage, fraction });

            let (_, digest) = self
                .download(&asset_url, &download_path, &|f| report(InstallStage::Downloading, f))
                .await?;

            report(InstallStage::Verifying, None);
            let sums = self.get_text(&sums_url).await?;
            let expected = find_checksum(&sums, asset.name())
                .ok_or_else(|| Error::Checksum(format!("{} 항목 없음", asset.name())))?;
            if !expected.eq_ignore_ascii_case(&digest) {
                return Err(Error::Checksum(asset.name().into()));
            }

            report(InstallStage::Installing, None);
            match asset {
                YtdlpAsset::Executable(_) => {
                    make_executable(&download_path).await?;
                    tokio::fs::rename(&download_path, &self.paths.ytdlp).await?;
                }
                YtdlpAsset::OnedirZip(_) => {
                    let out = staging.join("out");
                    extract_all(&download_path, &out).await?;
                    let new_bin = out.join(platform::YTDLP_BIN);
                    let new_internal = out.join(platform::YTDLP_INTERNAL_DIR);
                    if !new_bin.is_file() || !new_internal.is_dir() {
                        return Err(Error::Other("yt-dlp 압축 파일 구성이 예상과 다릅니다".into()));
                    }
                    make_executable(&new_bin).await?;
                    let internal = self.paths.bin_dir.join(platform::YTDLP_INTERNAL_DIR);
                    remove_path(&internal).await?;
                    remove_path(&self.paths.ytdlp).await?;
                    tokio::fs::rename(&new_internal, &internal).await?;
                    tokio::fs::rename(&new_bin, &self.paths.ytdlp).await?;
                }
            }
            Ok(())
        }
        .await;

        let _ = remove_path(&staging).await;
        result?;

        let version = self
            .local_ytdlp_version()
            .await
            .ok_or_else(|| Error::Other("설치한 yt-dlp를 실행하지 못했습니다".into()))?;
        on(InstallProgress { tool: Tool::Ytdlp, stage: InstallStage::Done, fraction: Some(1.0) });
        Ok(version)
    }

    /// ffmpeg 정적 빌드를 내려받아 실행 파일만 꺼내 설치한다.
    pub async fn install_ffmpeg(&self, on: ProgressFn<'_>) -> Result<()> {
        let source = platform::ffmpeg_source()?;
        tokio::fs::create_dir_all(&self.paths.bin_dir).await?;

        let staging = self.paths.bin_dir.join(".staging-ffmpeg");
        remove_path(&staging).await?;
        tokio::fs::create_dir_all(&staging).await?;

        let result = async {
            let report = |stage, fraction| on(InstallProgress { tool: Tool::Ffmpeg, stage, fraction });
            let zip_path = staging.join("ffmpeg.zip");

            let mut last_err = None;
            let mut downloaded = None;
            for url in source.urls {
                match self.download(url, &zip_path, &|f| report(InstallStage::Downloading, f)).await {
                    Ok(v) => {
                        downloaded = Some(v);
                        break;
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            let (final_url, digest) = match downloaded {
                Some(v) => v,
                None => return Err(last_err.unwrap_or_else(|| Error::Other("ffmpeg 배포처가 없습니다".into()))),
            };

            report(InstallStage::Verifying, None);
            let file_name = final_url.rsplit('/').next().unwrap_or_default().to_string();
            let sums = match source.checksum {
                ChecksumSource::Sibling(suffix) => self.get_text(&format!("{final_url}{suffix}")).await?,
                ChecksumSource::List(url) => self.get_text(url).await?,
            };
            let expected = find_checksum(&sums, &file_name)
                .ok_or_else(|| Error::Checksum(format!("{file_name} 항목 없음")))?;
            if !expected.eq_ignore_ascii_case(&digest) {
                return Err(Error::Checksum(file_name));
            }

            report(InstallStage::Installing, None);
            let tmp = staging.join(platform::FFMPEG_BIN);
            extract_one(&zip_path, source.entry_suffix, &tmp).await?;
            make_executable(&tmp).await?;
            tokio::fs::rename(&tmp, &self.paths.ffmpeg).await?;
            Ok(())
        }
        .await;

        let _ = remove_path(&staging).await;
        result?;
        on(InstallProgress { tool: Tool::Ffmpeg, stage: InstallStage::Done, fraction: Some(1.0) });
        Ok(())
    }

    async fn get_text(&self, url: &str) -> Result<String> {
        Ok(self
            .client
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    /// 파일을 내려받으며 SHA-256을 함께 계산한다. (리다이렉트 후 최종 URL, 해시) 반환.
    async fn download(&self, url: &str, dest: &Path, on: &(dyn Fn(Option<f64>) + Send + Sync)) -> Result<(String, String)> {
        let resp = self.client.get(url).send().await?.error_for_status()?;
        let final_url = resp.url().to_string();
        let total = resp.content_length().filter(|n| *n > 0);

        let mut file = tokio::fs::File::create(dest).await?;
        let mut hasher = Sha256::new();
        let mut received: u64 = 0;
        let mut last_reported = -1.0_f64;
        let mut stream = resp.bytes_stream();

        on(total.map(|_| 0.0));
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            received += chunk.len() as u64;
            if let Some(total) = total {
                let f = (received as f64 / total as f64).min(1.0);
                // 이벤트가 너무 많아지지 않도록 0.5% 단위로만 알린다.
                if f - last_reported >= 0.005 {
                    last_reported = f;
                    on(Some(f));
                }
            }
        }
        file.flush().await?;
        drop(file);
        Ok((final_url, hex::encode(hasher.finalize())))
    }
}

/// `a`가 `b`보다 새 버전인지. yt-dlp는 `2026.10.26`, `2026.10.26.1` 같은 날짜 형식을 쓴다.
pub fn is_newer(a: &str, b: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.trim_start_matches('v').split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect()
    }
    let (pa, pb) = (parts(a), parts(b));
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// `sha256sum` 형식(`<hash>  <name>` 또는 `<hash> *<name>`) 목록에서 해시를 찾는다.
/// 해시 하나만 적힌 파일이면 그 값을 쓴다.
pub fn find_checksum(sums: &str, file_name: &str) -> Option<String> {
    let is_hash = |s: &str| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit());
    let mut lone = None;
    for line in sums.lines() {
        let mut it = line.split_whitespace();
        let (Some(hash), name) = (it.next(), it.next()) else { continue };
        if !is_hash(hash) {
            continue;
        }
        match name {
            Some(n) if n.trim_start_matches('*') == file_name => return Some(hash.to_string()),
            None => lone = Some(hash.to_string()),
            _ => {}
        }
    }
    lone
}

async fn remove_path(path: &Path) -> Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(m) if m.is_dir() => Ok(tokio::fs::remove_dir_all(path).await?),
        Ok(_) => Ok(tokio::fs::remove_file(path).await?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

async fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).await?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// zip 전체를 푼다. macOS는 심볼릭 링크와 권한을 그대로 살리기 위해 `ditto`를 쓴다.
async fn extract_all(zip_path: &Path, dest: &Path) -> Result<()> {
    tokio::fs::create_dir_all(dest).await?;
    #[cfg(target_os = "macos")]
    {
        let out = tokio::process::Command::new("/usr/bin/ditto")
            .arg("-x")
            .arg("-k")
            .arg(zip_path)
            .arg(dest)
            .output()
            .await?;
        if !out.status.success() {
            return Err(Error::Other(format!(
                "압축 해제 실패: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (zip_path, dest) = (zip_path.to_path_buf(), dest.to_path_buf());
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path)?)?;
            archive.extract(dest)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }
}

/// zip 안에서 경로가 `suffix`로 끝나는 파일 하나만 `dest`로 꺼낸다.
async fn extract_one(zip_path: &Path, suffix: &str, dest: &Path) -> Result<()> {
    let (zip_path, suffix, dest) = (zip_path.to_path_buf(), suffix.to_string(), dest.to_path_buf());
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip_path)?)?;
        let index = (0..archive.len())
            .find(|&i| {
                archive
                    .name_for_index(i)
                    .and_then(|r| r.ok())
                    .map(|n| {
                        let n = n.replace('\\', "/");
                        !n.ends_with('/') && (n == suffix || n.ends_with(&format!("/{suffix}")))
                    })
                    .unwrap_or(false)
            })
            .ok_or_else(|| Error::Other(format!("압축 파일에서 {suffix}을(를) 찾지 못했습니다")))?;
        let mut entry = archive.by_index(index)?;
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Other(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("2026.10.26", "2026.09.30"));
        assert!(is_newer("2026.10.26.1", "2026.10.26"));
        assert!(!is_newer("2026.10.26", "2026.10.26"));
        assert!(!is_newer("2025.12.01", "2026.01.01"));
    }

    #[test]
    fn checksum_lookup() {
        let sums = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  yt-dlp.exe\n\
                    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb *yt-dlp_macos.zip\n";
        assert_eq!(find_checksum(sums, "yt-dlp_macos.zip").unwrap().chars().next(), Some('b'));
        assert_eq!(find_checksum(sums, "yt-dlp.exe").unwrap().chars().next(), Some('a'));
        assert!(find_checksum(sums, "nope").is_none());
        let lone = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\n";
        assert!(find_checksum(lone, "ffmpeg.zip").is_some());
    }
}
