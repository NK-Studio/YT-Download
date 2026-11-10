// Windows 릴리스 빌드에서 콘솔 창이 뜨지 않게 한다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::{Arc, Mutex};

use nupy_core::{ToolManager, ToolPaths};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub tools: Arc<ToolManager>,
    /// 쓰기 잠금 = yt-dlp/ffmpeg 설치·교체, 읽기 잠금 = yt-dlp 실행.
    /// 실행 중인 바이너리를 덮어쓰지 않도록 한다 (Windows는 실행 중인 exe를 교체할 수 없다).
    pub binaries: RwLock<()>,
    pub download: Mutex<Option<CancellationToken>>,
}

fn main() {
    let paths = ToolPaths::default_location().expect("사용자 데이터 폴더를 찾지 못했습니다");
    let tools = ToolManager::new(paths).expect("HTTP 클라이언트를 만들지 못했습니다");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            tools: Arc::new(tools),
            binaries: RwLock::new(()),
            download: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::tool_status,
            commands::prepare_tools,
            commands::update_ytdlp,
            commands::fetch_info,
            commands::start_download,
            commands::cancel_download,
            commands::system_info,
        ])
        .run(tauri::generate_context!())
        .expect("앱을 실행하지 못했습니다");
}
