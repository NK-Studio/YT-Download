using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace NupyYTDownloader.Models;

// ── yt-dlp JSON 파싱용 구조체 ──────────────────────────────────────────────

public class YtdlpResponse
{
    [JsonPropertyName("title")]
    public string Title { get; set; } = "";

    [JsonPropertyName("thumbnail")]
    public string Thumbnail { get; set; } = "";

    [JsonPropertyName("formats")]
    public List<YtdlpFormat> Formats { get; set; } = new();
}

public class YtdlpFormat
{
    [JsonPropertyName("format_id")]
    public string FormatId { get; set; } = "";

    [JsonPropertyName("ext")]
    public string Ext { get; set; } = "";

    [JsonPropertyName("width")]
    public int? Width { get; set; }

    [JsonPropertyName("height")]
    public int? Height { get; set; }

    [JsonPropertyName("filesize")]
    public long? Filesize { get; set; }

    [JsonPropertyName("filesize_approx")]
    public long? FilesizeApprox { get; set; }

    [JsonPropertyName("vcodec")]
    public string? Vcodec { get; set; }

    [JsonPropertyName("acodec")]
    public string? Acodec { get; set; }
}

// ── 앱 내부 모델 ────────────────────────────────────────────────────────────

public class VideoMetadata
{
    public string Title { get; set; } = "";
    public string ThumbnailUrl { get; set; } = "";
}

public class VideoFormat
{
    public string Id { get; set; } = "";
    public string Ext { get; set; } = "";
    public string Resolution { get; set; } = "";
    public string FileSize { get; set; } = "";
    public string Codec { get; set; } = "";
    public string Vcodec { get; set; } = "";

    /// <summary>오디오 전용 포맷 여부 (vcodec == "none" 이거나 height 없음)</summary>
    public bool IsAudioOnly { get; set; }

    /// <summary>UI 표시용 확장자 — 오디오는 항상 mp3로 표기</summary>
    public string DisplayExt => IsAudioOnly ? "mp3" : Ext;

    /// <summary>코덱 태그 — 포맷 카드에 표시</summary>
    public string CodecTag
    {
        get
        {
            if (IsAudioOnly) return "MP3 오디오";
            if (Codec.Contains("avc1")) return "H.264 (호환성↑)";
            if (Codec.Contains("av01")) return "AV1 (고화질)";
            if (Codec.Contains("vp9")) return "VP9 (고화질)";
            return "일반";
        }
    }
}