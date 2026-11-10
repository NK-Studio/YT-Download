use serde::{Serialize, Serializer};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("네트워크 오류: {0}")]
    Http(#[from] reqwest::Error),
    #[error("파일 오류: {0}")]
    Io(#[from] std::io::Error),
    #[error("압축 해제 오류: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("응답을 해석하지 못했습니다: {0}")]
    Json(#[from] serde_json::Error),
    #[error("내려받은 파일의 체크섬이 일치하지 않습니다 ({0})")]
    Checksum(String),
    #[error("yt-dlp가 설치되어 있지 않습니다")]
    YtdlpMissing,
    #[error("ffmpeg가 설치되어 있지 않습니다")]
    FfmpegMissing,
    #[error("{0}")]
    Ytdlp(String),
    #[error("취소되었습니다")]
    Cancelled,
    #[error("지원하지 않는 플랫폼입니다 ({0})")]
    UnsupportedPlatform(&'static str),
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Error::Cancelled)
    }
}

// Tauri 명령에서 그대로 반환할 수 있도록 메시지 문자열로 직렬화한다.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
