use std::path::PathBuf;

use nupy_core::{ytdlp, DownloadEvent, DownloadRequest, Error, InstallProgress, ToolStatus, VideoInfo};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio_util::sync::CancellationToken;

use crate::AppState;

type CmdResult<T> = Result<T, Error>;

/// 현재 설치 상태. `check_latest`면 GitHub 최신 버전도 조회한다.
#[tauri::command]
pub async fn tool_status(state: State<'_, AppState>, check_latest: bool) -> CmdResult<ToolStatus> {
    let _guard = state.binaries.read().await;
    Ok(state.tools.status(check_latest).await)
}

/// 앱 시작 시 호출한다.
/// 1. yt-dlp / ffmpeg 가 없으면 내려받는다 (첫 실행).
/// 2. GitHub 최신 yt-dlp 가 더 새것이면 `auto_update`일 때 바로 교체한다.
///    업데이트 실패는 치명적이지 않으므로 기존 버전을 유지하고 `update_available`로 알린다.
#[tauri::command]
pub async fn prepare_tools(
    state: State<'_, AppState>,
    auto_update: bool,
    on_progress: Channel<InstallProgress>,
) -> CmdResult<ToolStatus> {
    let _guard = state.binaries.write().await;
    let tools = &state.tools;
    let report = |p: InstallProgress| {
        let _ = on_progress.send(p);
    };

    let latest = tools.latest_ytdlp_version().await.ok();

    if !tools.paths().ytdlp_installed() || tools.local_ytdlp_version().await.is_none() {
        tools.install_ytdlp(latest.as_deref(), &report).await?;
    } else if auto_update {
        let current = tools.local_ytdlp_version().await;
        if let (Some(l), Some(c)) = (&latest, &current) {
            if nupy_core::tools::is_newer(l, c) {
                // 실패해도 기존 yt-dlp 로 계속 쓸 수 있다.
                let _ = tools.install_ytdlp(Some(l), &report).await;
            }
        }
    }

    if !tools.paths().ffmpeg_installed() {
        tools.install_ffmpeg(&report).await?;
    }

    let mut status = tools.status(false).await;
    status.update_available =
        matches!((&latest, &status.ytdlp_version), (Some(l), Some(c)) if nupy_core::tools::is_newer(l, c));
    status.latest_ytdlp_version = latest;
    Ok(status)
}

/// 사용자가 업데이트 버튼을 눌렀을 때: yt-dlp 를 최신 릴리스로 교체한다.
#[tauri::command]
pub async fn update_ytdlp(state: State<'_, AppState>, on_progress: Channel<InstallProgress>) -> CmdResult<ToolStatus> {
    if state.download.lock().unwrap().is_some() {
        return Err(Error::Other("다운로드가 끝난 뒤에 업데이트해 주세요".into()));
    }
    let _guard = state.binaries.write().await;
    let latest = state.tools.latest_ytdlp_version().await.ok();
    state
        .tools
        .install_ytdlp(latest.as_deref(), &|p| {
            let _ = on_progress.send(p);
        })
        .await?;
    if !state.tools.paths().ffmpeg_installed() {
        state
            .tools
            .install_ffmpeg(&|p| {
                let _ = on_progress.send(p);
            })
            .await?;
    }
    Ok(state.tools.status(true).await)
}

#[tauri::command]
pub async fn fetch_info(state: State<'_, AppState>, url: String) -> CmdResult<VideoInfo> {
    let url = url.trim();
    if url.is_empty() {
        return Err(Error::Other("링크를 입력해 주세요".into()));
    }
    let _guard = state.binaries.read().await;
    ytdlp::fetch_info(state.tools.paths(), url).await
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum DownloadOutcome {
    Finished { path: PathBuf },
    Cancelled,
}

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    request: DownloadRequest,
    on_event: Channel<DownloadEvent>,
) -> CmdResult<DownloadOutcome> {
    let token = CancellationToken::new();
    {
        let mut slot = state.download.lock().unwrap();
        if slot.is_some() {
            return Err(Error::Other("이미 다운로드가 진행 중입니다".into()));
        }
        *slot = Some(token.clone());
    }

    let result = {
        let _guard = state.binaries.read().await;
        ytdlp::download(state.tools.paths(), &request, token, |ev| {
            let _ = on_event.send(ev);
        })
        .await
    };
    state.download.lock().unwrap().take();

    match result {
        Ok(path) => Ok(DownloadOutcome::Finished { path }),
        Err(e) if e.is_cancelled() => Ok(DownloadOutcome::Cancelled),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Some(token) = state.download.lock().unwrap().as_ref() {
        token.cancel();
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub app_version: &'static str,
    /// Windows 11(빌드 22000 이상)에서만 Mica 배경을 쓸 수 있다.
    pub supports_mica: bool,
}

#[tauri::command]
pub fn system_info() -> SystemInfo {
    #[cfg(windows)]
    let supports_mica = windows_version::OsVersion::current().build >= 22000;
    #[cfg(not(windows))]
    let supports_mica = false;
    SystemInfo { app_version: env!("CARGO_PKG_VERSION"), supports_mica }
}
