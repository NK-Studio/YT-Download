import Foundation

// yt-dlp JSON 파싱을 위한 구조체
struct YtdlpResponse: Codable {
    let title: String
    let thumbnail: String
    let formats: [YtdlpFormat]
}

struct YtdlpFormat: Codable {
    let format_id: String
    let ext: String
    let width: Int?
    let height: Int?
    let filesize: Int64?
    let vcodec: String?
    let acodec: String?
}

// 앱 내에서 사용할 모델
struct VideoMetadata {
    let title: String
    let thumbnailUrl: String
}

struct VideoFormat: Identifiable {
    let id: String
    let ext: String
    let resolution: String
    let fileSize: String
    let codec: String
    let vcodec: String // 실제 명령용
    
    // UI 표시용 로직
    var codecTag: String {
        if vcodec == "none" && codec != "audio only" { return "오디오 전용" }
        if codec.contains("avc1") { return "H.264 (호환성↑)" }
        if codec.contains("av01") || codec.contains("vp9") { return "VP9/AV1 (고화질)" }
        return "일반"
    }
}
