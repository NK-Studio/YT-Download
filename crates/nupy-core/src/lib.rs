//! YT Downloader 코어.
//!
//! UI(Tauri)와 독립적으로 다음을 담당한다.
//! - yt-dlp / ffmpeg 바이너리를 사용자 데이터 폴더에 설치하고 최신 릴리스로 교체 ([`tools`])
//! - yt-dlp로 영상 정보 조회와 다운로드, 진행률 파싱 ([`ytdlp`])
//!
//! yt-dlp는 앱 번들에 넣지 않고 런타임에 내려받는다. 그래서 YouTube 쪽 변경으로
//! yt-dlp가 업데이트되어도 앱을 다시 빌드할 필요가 없다.

pub mod error;
pub mod platform;
mod process;
pub mod tools;
pub mod ytdlp;

pub use error::{Error, Result};
pub use tools::{InstallProgress, InstallStage, Tool, ToolManager, ToolPaths, ToolStatus};
pub use ytdlp::{DownloadEvent, DownloadRequest, FormatKind, FormatOption, VideoInfo};
