//! yt-dlp 실행: 영상 정보 조회, 다운로드, 진행률 파싱.
//!
//! 오디오는 받은 스트림을 그대로 저장한다(m4a/webm). MP3 변환은 하지 않는다.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::process::{command, kill_tree};
use crate::tools::ToolPaths;
use crate::{Error, Result};

/// 진행률 줄과 최종 경로 줄을 일반 로그와 구분하기 위한 표식.
const PROGRESS_MARK: &str = "__NUPY_PROGRESS__";
const FILE_MARK: &str = "__NUPY_FILE__";

/// 모든 실행에 공통으로 붙이는 인자. 사용자의 yt-dlp 설정 파일은 무시한다.
const COMMON_ARGS: &[&str] = &["--ignore-config", "--no-playlist", "--no-colors", "--encoding", "utf-8"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FormatKind {
    /// 영상 + 오디오가 한 파일에 들어 있음
    Muxed,
    /// 영상만 있음 → 최적 오디오와 합쳐 mp4로 저장
    VideoOnly,
    /// 오디오만 있음 → 원본 그대로 저장
    AudioOnly,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatOption {
    pub id: String,
    pub kind: FormatKind,
    /// yt-dlp가 알려준 원본 확장자
    pub ext: String,
    /// 저장될 파일의 확장자 (VideoOnly는 mp4로 합친다)
    pub output_ext: String,
    /// "1080p60", "128 kbps" 같은 표시용 이름
    pub label: String,
    /// "H.264", "AV1", "AAC" 같은 코덱 이름
    pub codec_label: String,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub abr: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize: Option<u64>,
    /// `filesize`가 추정치인지
    pub filesize_approx: bool,
    pub dynamic_range: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub webpage_url: Option<String>,
    pub formats: Vec<FormatOption>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub format_id: String,
    pub kind: FormatKind,
    /// 저장 대화상자에서 고른 경로. 확장자는 형식에 맞게 다시 정해진다.
    pub output: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DownloadEvent {
    Started {
        stages: u8,
    },
    Progress {
        /// 0부터 시작하는 단계 번호 (VideoOnly는 0=영상, 1=오디오)
        stage: u8,
        stages: u8,
        /// 전체 진행률 0.0~1.0
        overall: f64,
        downloaded_bytes: Option<u64>,
        total_bytes: Option<u64>,
        /// 바이트/초
        speed: Option<f64>,
        /// 남은 초
        eta: Option<u64>,
    },
    /// 영상과 오디오를 합치는 중
    Merging,
    Finished {
        path: PathBuf,
    },
}

// ───────────────────────── 영상 정보 ─────────────────────────

#[derive(Deserialize)]
struct RawInfo {
    id: String,
    title: String,
    uploader: Option<String>,
    channel: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    webpage_url: Option<String>,
    #[serde(default)]
    formats: Vec<RawFormat>,
}

#[derive(Deserialize)]
struct RawFormat {
    format_id: String,
    ext: String,
    height: Option<u32>,
    fps: Option<f64>,
    abr: Option<f64>,
    tbr: Option<f64>,
    vcodec: Option<String>,
    acodec: Option<String>,
    filesize: Option<u64>,
    filesize_approx: Option<u64>,
    protocol: Option<String>,
    format_note: Option<String>,
    dynamic_range: Option<String>,
    language: Option<String>,
}

pub async fn fetch_info(paths: &ToolPaths, url: &str) -> Result<VideoInfo> {
    if !paths.ytdlp_installed() {
        return Err(Error::YtdlpMissing);
    }
    let out = command(&paths.ytdlp)
        .args(COMMON_ARGS)
        .arg("--ffmpeg-location")
        .arg(&paths.ffmpeg)
        .args(["-J", "--", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !out.status.success() {
        return Err(Error::Ytdlp(error_message(&String::from_utf8_lossy(&out.stderr))));
    }
    let raw: RawInfo = serde_json::from_slice(&out.stdout)?;
    Ok(VideoInfo {
        formats: build_formats(raw.formats),
        id: raw.id,
        title: raw.title,
        uploader: raw.uploader.or(raw.channel),
        duration: raw.duration,
        thumbnail: raw.thumbnail,
        webpage_url: raw.webpage_url,
    })
}

fn is_none(codec: &Option<String>) -> bool {
    codec.as_deref().is_none_or(|c| c == "none")
}

fn codec_label(codec: &str) -> &'static str {
    let c = codec.to_ascii_lowercase();
    match () {
        _ if c.starts_with("avc") || c.starts_with("h264") => "H.264",
        _ if c.starts_with("hev") || c.starts_with("hvc") || c.starts_with("h265") => "HEVC",
        _ if c.starts_with("av01") || c == "av1" => "AV1",
        _ if c.starts_with("vp09") || c.starts_with("vp9") => "VP9",
        _ if c.starts_with("vp8") => "VP8",
        _ if c.starts_with("mp4a") || c == "aac" => "AAC",
        _ if c.starts_with("opus") => "Opus",
        _ if c.starts_with("vorbis") => "Vorbis",
        _ if c.starts_with("mp3") => "MP3",
        _ if c.starts_with("ac-3") || c.starts_with("ec-3") => "Dolby",
        _ => "기타",
    }
}

/// yt-dlp의 포맷 목록을 화면에 보여줄 목록으로 정리한다.
/// - 스토리보드 등 영상/오디오가 없는 항목 제거
/// - 같은 (종류, 해상도/비트레이트, 코덱, 확장자)는 https 직접 다운로드와 높은 비트레이트를 우선해 하나만 남김
/// - 영상은 해상도 내림차순, 오디오는 비트레이트 내림차순
fn build_formats(raw: Vec<RawFormat>) -> Vec<FormatOption> {
    let mut best: HashMap<String, (FormatOption, (bool, i64))> = HashMap::new();

    for f in raw {
        if f.ext == "mhtml" || f.format_note.as_deref() == Some("storyboard") {
            continue;
        }
        let has_video = !is_none(&f.vcodec) || (f.vcodec.is_none() && f.height.is_some());
        let has_audio = !is_none(&f.acodec);
        let kind = match (has_video, has_audio) {
            (true, true) => FormatKind::Muxed,
            (true, false) => FormatKind::VideoOnly,
            (false, true) => FormatKind::AudioOnly,
            (false, false) => continue,
        };

        let fps = f.fps.filter(|v| *v > 0.0);
        let label = match kind {
            FormatKind::AudioOnly => match f.abr.or(f.tbr) {
                Some(abr) => format!("{} kbps", abr.round() as u32),
                None => "오디오".into(),
            },
            _ => match f.height {
                Some(h) => match fps {
                    Some(fps) if fps > 30.5 => format!("{h}p{}", fps.round() as u32),
                    _ => format!("{h}p"),
                },
                None => f.format_note.clone().unwrap_or_else(|| "영상".into()),
            },
        };
        let codec_label = match kind {
            FormatKind::AudioOnly => codec_label(f.acodec.as_deref().unwrap_or("")).to_string(),
            _ => codec_label(f.vcodec.as_deref().unwrap_or("")).to_string(),
        };
        let output_ext = match kind {
            FormatKind::VideoOnly => "mp4".to_string(),
            _ => f.ext.clone(),
        };
        let (filesize, filesize_approx) = match (f.filesize, f.filesize_approx) {
            (Some(s), _) => (Some(s), false),
            (None, Some(s)) => (Some(s), true),
            _ => (None, false),
        };
        let dynamic_range = f.dynamic_range.clone().filter(|d| d != "SDR");

        // 영상 전용은 어차피 mp4로 합쳐지므로 원본 확장자(HLS의 mp4 / DASH의 webm 등)로 나누지 않는다.
        let key = format!(
            "{kind:?}|{label}|{codec_label}|{}|{}|{}",
            if kind == FormatKind::VideoOnly { "" } else { &f.ext },
            dynamic_range.as_deref().unwrap_or(""),
            f.language.as_deref().unwrap_or("")
        );
        let is_direct = !f.protocol.as_deref().unwrap_or("https").starts_with("m3u8");
        let score = (is_direct, (f.tbr.unwrap_or(0.0) * 1000.0) as i64);

        let option = FormatOption {
            id: f.format_id,
            kind,
            ext: f.ext,
            output_ext,
            label,
            codec_label,
            height: f.height,
            fps,
            abr: f.abr,
            vcodec: f.vcodec.filter(|c| c != "none"),
            acodec: f.acodec.filter(|c| c != "none"),
            filesize,
            filesize_approx,
            dynamic_range,
        };
        match best.get(&key) {
            Some((_, s)) if *s >= score => {}
            _ => {
                best.insert(key, (option, score));
            }
        }
    }

    let mut list: Vec<FormatOption> = best.into_values().map(|(o, _)| o).collect();
    list.sort_by(|a, b| {
        let group = |f: &FormatOption| (f.kind == FormatKind::AudioOnly) as u8;
        group(a)
            .cmp(&group(b))
            .then(b.height.unwrap_or(0).cmp(&a.height.unwrap_or(0)))
            .then(b.fps.unwrap_or(0.0).total_cmp(&a.fps.unwrap_or(0.0)))
            .then(b.abr.unwrap_or(0.0).total_cmp(&a.abr.unwrap_or(0.0)))
            .then(b.filesize.unwrap_or(0).cmp(&a.filesize.unwrap_or(0)))
    });
    list
}

// ───────────────────────── 다운로드 ─────────────────────────

/// 선택한 형식을 내려받는다. 취소되면 [`Error::Cancelled`]를 돌려주고 임시 파일을 지운다.
pub async fn download(
    paths: &ToolPaths,
    req: &DownloadRequest,
    cancel: CancellationToken,
    mut on_event: impl FnMut(DownloadEvent) + Send,
) -> Result<PathBuf> {
    if !paths.ytdlp_installed() {
        return Err(Error::YtdlpMissing);
    }
    if req.kind == FormatKind::VideoOnly && !paths.ffmpeg_installed() {
        return Err(Error::FfmpegMissing);
    }

    let dir = req
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| Error::Other("저장 위치가 올바르지 않습니다".into()))?
        .to_path_buf();
    let stem = req
        .output
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Other("파일 이름이 비어 있습니다".into()))?;

    // yt-dlp의 -o 는 출력 템플릿이므로 파일 이름의 % 를 이스케이프하고,
    // 확장자는 실제로 저장되는 형식에 맞게 yt-dlp가 붙이도록 한다.
    let template = dir.join(format!("{}.%(ext)s", stem.replace('%', "%%")));

    let (selector, stages): (String, u8) = match req.kind {
        FormatKind::VideoOnly => (format!("{0}+bestaudio[ext=m4a]/{0}+bestaudio", req.format_id), 2),
        _ => (req.format_id.clone(), 1),
    };

    let mut cmd = command(&paths.ytdlp);
    cmd.args(COMMON_ARGS)
        .args(["--newline", "--progress", "--force-overwrites", "--no-mtime"])
        .arg("--ffmpeg-location")
        .arg(&paths.ffmpeg)
        .args(["-f", &selector]);
    if req.kind == FormatKind::VideoOnly {
        cmd.args(["--merge-output-format", "mp4"]);
    }
    cmd.arg("--progress-template")
        .arg(format!(
            "download:{PROGRESS_MARK} %(progress.status)s %(progress.downloaded_bytes)s %(progress.total_bytes)s \
             %(progress.total_bytes_estimate)s %(progress.speed)s %(progress.eta)s %(info.format_id)s"
        ))
        .args(["--print", &format!("after_move:{FILE_MARK}%(filepath)s")])
        .arg("-o")
        .arg(&template)
        .args(["--", &req.url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let pid = child.id();

    // stdout/stderr를 함께 읽어야 파이프가 가득 차 멈추는 일이 없다.
    let (tx, mut rx) = mpsc::unbounded_channel::<Line>();
    spawn_reader(child.stdout.take(), false, tx.clone());
    spawn_reader(child.stderr.take(), true, tx);

    on_event(DownloadEvent::Started { stages });

    let mut tracker = ProgressTracker::new(&req.format_id, stages);
    let mut final_path: Option<PathBuf> = None;
    let mut errors: Vec<String> = Vec::new();
    let mut cancelled = false;

    loop {
        tokio::select! {
            line = rx.recv() => {
                let Some(line) = line else { break };
                if let Some(rest) = line.text.strip_prefix(FILE_MARK) {
                    final_path = Some(PathBuf::from(rest.trim()));
                } else if let Some(ev) = tracker.parse(&line.text) {
                    on_event(ev);
                } else if line.is_stderr && line.text.starts_with("ERROR:") {
                    errors.push(line.text.trim_start_matches("ERROR:").trim().to_string());
                }
            }
            _ = cancel.cancelled(), if !cancelled => {
                cancelled = true;
                if let Some(pid) = pid { kill_tree(pid); }
                let _ = child.start_kill();
            }
        }
    }

    let status = child.wait().await?;
    if cancelled {
        cleanup_partials(&dir, &stem).await;
        return Err(Error::Cancelled);
    }
    if !status.success() {
        cleanup_partials(&dir, &stem).await;
        let msg = if errors.is_empty() {
            format!("yt-dlp가 비정상 종료했습니다 (코드 {})", status.code().unwrap_or(-1))
        } else {
            errors.join("\n")
        };
        return Err(Error::Ytdlp(msg));
    }

    let path = final_path.unwrap_or_else(|| req.output.clone());
    on_event(DownloadEvent::Finished { path: path.clone() });
    Ok(path)
}

struct Line {
    text: String,
    is_stderr: bool,
}

fn spawn_reader<R>(stream: Option<R>, is_stderr: bool, tx: mpsc::UnboundedSender<Line>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let Some(stream) = stream else { return };
    tokio::spawn(async move {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    // --newline 이 없어도 \r 로 덮어쓰는 진행률 줄을 나눠 읽는다.
                    let text = String::from_utf8_lossy(&buf);
                    for part in text.split(['\r', '\n']).map(str::trim).filter(|s| !s.is_empty()) {
                        let _ = tx.send(Line { text: part.to_string(), is_stderr });
                    }
                }
            }
        }
    });
}

/// 진행률 템플릿 줄을 전체 진행률로 바꾼다.
///
/// 영상만 있는 형식은 yt-dlp가 영상 → 오디오 순서로 각각 0~100%를 내보내므로
/// `info.format_id`로 지금 받는 스트림을 구분해 0~50%, 50~100%로 합친다.
struct ProgressTracker {
    video_format_id: String,
    stages: u8,
    last_overall: f64,
    merging_sent: bool,
}

impl ProgressTracker {
    fn new(format_id: &str, stages: u8) -> Self {
        Self { video_format_id: format_id.to_string(), stages, last_overall: 0.0, merging_sent: false }
    }

    fn parse(&mut self, line: &str) -> Option<DownloadEvent> {
        if line.starts_with("[Merger]") || line.starts_with("[VideoRemuxer]") {
            return self.merging();
        }
        let rest = line.strip_prefix(PROGRESS_MARK)?;
        let mut it = rest.split_whitespace();
        let status = it.next()?;
        let num = |s: Option<&str>| s.and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v >= 0.0);
        let downloaded = num(it.next());
        let total = num(it.next());
        let estimate = num(it.next());
        let speed = num(it.next());
        let eta = num(it.next());
        let fid = it.next().unwrap_or("");

        let stage: u8 = if self.stages > 1 && fid != self.video_format_id { 1 } else { 0 };
        let total = total.or(estimate);
        let fraction = match (status, downloaded, total) {
            ("finished", _, _) => 1.0,
            (_, Some(d), Some(t)) if t > 0.0 => (d / t).min(1.0),
            _ => 0.0,
        };
        let overall = ((stage as f64 + fraction) / self.stages as f64).clamp(0.0, 1.0);
        // 추정치 변동으로 진행률이 뒤로 가지 않게 한다.
        let overall = overall.max(self.last_overall);
        self.last_overall = overall;

        if status == "finished" && stage + 1 == self.stages && self.stages > 1 {
            return self.merging();
        }
        Some(DownloadEvent::Progress {
            stage,
            stages: self.stages,
            overall,
            downloaded_bytes: downloaded.map(|v| v as u64),
            total_bytes: total.map(|v| v as u64),
            speed,
            eta: eta.map(|v| v as u64),
        })
    }

    fn merging(&mut self) -> Option<DownloadEvent> {
        if self.merging_sent {
            return None;
        }
        self.merging_sent = true;
        Some(DownloadEvent::Merging)
    }
}

/// yt-dlp가 남긴 `.part`, `.ytdl`, `name.f137.webm` 같은 중간 파일만 지운다.
async fn cleanup_partials(dir: &Path, stem: &str) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else { return };
    let prefix = format!("{stem}.");
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix(&prefix) else { continue };
        let is_partial = rest.ends_with(".part")
            || rest.ends_with(".ytdl")
            || rest.contains(".part-Frag")
            || is_intermediate(rest);
        if is_partial {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

/// `f137.mp4`, `f251.webm.part` 처럼 형식 번호가 붙은 중간 파일인지.
fn is_intermediate(rest: &str) -> bool {
    let Some(r) = rest.strip_prefix('f') else { return false };
    let id: String = r.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    !id.is_empty() && r[id.len()..].starts_with('.') && id.chars().any(|c| c.is_ascii_digit())
}

/// stderr에서 사용자에게 보여줄 오류 메시지를 고른다.
fn error_message(stderr: &str) -> String {
    let errors: Vec<&str> = stderr
        .lines()
        .filter_map(|l| l.trim().strip_prefix("ERROR:"))
        .map(str::trim)
        .collect();
    if !errors.is_empty() {
        return errors.join("\n");
    }
    stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("알 수 없는 오류").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(id: &str, ext: &str, h: Option<u32>, v: &str, a: &str, tbr: f64) -> RawFormat {
        RawFormat {
            format_id: id.into(),
            ext: ext.into(),
            height: h,
            fps: Some(30.0),
            abr: if v == "none" { Some(tbr) } else { None },
            tbr: Some(tbr),
            vcodec: Some(v.into()),
            acodec: Some(a.into()),
            filesize: Some(1000),
            filesize_approx: None,
            protocol: Some("https".into()),
            format_note: None,
            dynamic_range: None,
            language: None,
        }
    }

    #[test]
    fn formats_are_classified_and_sorted() {
        let list = build_formats(vec![
            raw("sb0", "mhtml", None, "none", "none", 0.0),
            raw("140", "m4a", None, "none", "mp4a.40.2", 129.0),
            raw("18", "mp4", Some(360), "avc1.42001E", "mp4a.40.2", 500.0),
            raw("137", "mp4", Some(1080), "avc1.640028", "none", 4000.0),
            raw("399", "mp4", Some(1080), "av01.0.08M.08", "none", 2000.0),
        ]);
        let ids: Vec<_> = list.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(list.len(), 4);
        assert_eq!(list.last().unwrap().kind, FormatKind::AudioOnly);
        assert!(ids[..2].contains(&"137") && ids[..2].contains(&"399"));
        let v = list.iter().find(|f| f.id == "137").unwrap();
        assert_eq!(v.kind, FormatKind::VideoOnly);
        assert_eq!(v.output_ext, "mp4");
        assert_eq!(v.codec_label, "H.264");
        assert_eq!(list.iter().find(|f| f.id == "18").unwrap().kind, FormatKind::Muxed);
    }

    #[test]
    fn progress_two_stages() {
        let mut t = ProgressTracker::new("137", 2);
        let ev = t.parse(&format!("{PROGRESS_MARK} downloading 50 100 NA 10.0 5 137")).unwrap();
        match ev {
            DownloadEvent::Progress { overall, stage, .. } => {
                assert_eq!(stage, 0);
                assert!((overall - 0.25).abs() < 1e-9);
            }
            _ => panic!(),
        }
        let ev = t.parse(&format!("{PROGRESS_MARK} downloading 50 NA 100 NA NA 140")).unwrap();
        match ev {
            DownloadEvent::Progress { overall, stage, .. } => {
                assert_eq!(stage, 1);
                assert!((overall - 0.75).abs() < 1e-9);
            }
            _ => panic!(),
        }
        assert!(matches!(t.parse(&format!("{PROGRESS_MARK} finished 100 100 NA NA NA 140")), Some(DownloadEvent::Merging)));
        assert!(t.parse("[Merger] Merging formats into \"x.mp4\"").is_none());
    }

    #[test]
    fn intermediate_files() {
        assert!(is_intermediate("f137.mp4"));
        assert!(is_intermediate("f251-drc.webm.part"));
        assert!(!is_intermediate("mp4"));
        assert!(!is_intermediate("final.mp4"));
    }

    #[test]
    fn picks_error_lines() {
        let s = "WARNING: foo\nERROR: [youtube] abc: Video unavailable\n";
        assert_eq!(error_message(s), "[youtube] abc: Video unavailable");
    }
}
