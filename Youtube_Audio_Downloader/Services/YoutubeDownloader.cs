using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using NupyYTDownloader.Models;

namespace NupyYTDownloader.Services;

public class YoutubeDownloader : INotifyPropertyChanged
{
    // ── 상태 프로퍼티 ────────────────────────────────────────────────────────

    private double _downloadProgress;
    public double DownloadProgress
    {
        get => _downloadProgress;
        set { _downloadProgress = value; OnPropertyChanged(); }
    }

    private bool _isDownloading;
    public bool IsDownloading
    {
        get => _isDownloading;
        set { _isDownloading = value; OnPropertyChanged(); }
    }

    private string _currentStatus = "";
    public string CurrentStatus
    {
        get => _currentStatus;
        set { _currentStatus = value; OnPropertyChanged(); }
    }

    // ── 실행 파일 경로 ───────────────────────────────────────────────────────

    private readonly string _ytdlpPath;
    private readonly string _ffmpegDir; // yt-dlp는 --ffmpeg-location에 디렉터리를 받음

    public YoutubeDownloader(string ytdlpPath, string ffmpegPath)
    {
        _ytdlpPath = ytdlpPath;
        // ffmpeg.exe 경로에서 디렉터리만 추출
        _ffmpegDir = Path.GetDirectoryName(ffmpegPath) ?? Path.GetDirectoryName(ytdlpPath)!;
    }

    // ── 1. 영상 정보 가져오기 ────────────────────────────────────────────────

    /// <summary>
    /// yt-dlp -j 로 영상 메타데이터 및 포맷 목록을 가져옵니다.
    /// stderr를 동시에 읽어 데드락을 방지합니다.
    /// </summary>
    public async Task<(VideoMetadata Metadata, List<VideoFormat> Formats)> FetchVideoInfoAsync(string url)
    {
        var psi = new ProcessStartInfo
        {
            FileName = _ytdlpPath,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };
        psi.ArgumentList.Add("-j");
        psi.ArgumentList.Add(url);

        using var process = new Process { StartInfo = psi };
        process.Start();

        // ★ stdout과 stderr를 동시에 읽어야 데드락 없음
        var stdoutTask = process.StandardOutput.ReadToEndAsync();
        var stderrTask = process.StandardError.ReadToEndAsync();
        await process.WaitForExitAsync();

        var stdout = await stdoutTask;
        var stderr = await stderrTask;

        if (process.ExitCode != 0)
            throw new Exception($"yt-dlp 오류:\n{stderr.Trim()}");

        var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true };
        var info = JsonSerializer.Deserialize<YtdlpResponse>(stdout, options)
                      ?? throw new Exception("JSON 파싱에 실패했습니다.");

        var metadata = new VideoMetadata
        {
            Title = info.Title,
            ThumbnailUrl = info.Thumbnail
        };

        var formats = new List<VideoFormat>();
        foreach (var f in info.Formats)
        {
            // ★ filesize가 없으면 filesize_approx으로 대체 — 없으면 표시용 "? MB"
            long? size = f.Filesize ?? f.FilesizeApprox;
            string sizeStr = size is > 0
                ? $"{(double)size / 1024 / 1024:F1} MB"
                : "? MB";

            bool isAudio = f.Vcodec == "none" || f.Height == null;
            string resolution = isAudio ? "Audio" : $"{f.Height}p";
            string codec = f.Vcodec ?? "none";

            formats.Add(new VideoFormat
            {
                Id = f.FormatId,
                Ext = f.Ext,
                Resolution = resolution,
                FileSize = sizeStr,
                Codec = codec,
                Vcodec = codec,
                IsAudioOnly = isAudio
            });
        }

        formats.Reverse(); // 고화질이 보통 목록 뒤쪽에 있으므로 역순
        return (metadata, formats);
    }

    // ── 2-A. 비디오 다운로드 ─────────────────────────────────────────────────

    // ── 2-A. 비디오 다운로드 ─────────────────────────────────────────────────

    public async Task DownloadVideoAsync(string url, string formatId, string outputPath)
    {
        IsDownloading = true;
        DownloadProgress = 0;
        CurrentStatus = "다운로드 시작...";
        _isAudioOnlyDownload = false;  // ★ 영상 다운로드 플래그

        var psi = BuildBasePsi();

        psi.ArgumentList.Add("-f");
        psi.ArgumentList.Add($"{formatId}+bestaudio/best");
        psi.ArgumentList.Add("--merge-output-format");
        psi.ArgumentList.Add("mp4");
        psi.ArgumentList.Add("--ffmpeg-location");
        psi.ArgumentList.Add(_ffmpegDir);
        psi.ArgumentList.Add("--force-overwrites");
        psi.ArgumentList.Add("--no-colors");
        psi.ArgumentList.Add("--newline");
        psi.ArgumentList.Add("-o");
        psi.ArgumentList.Add(outputPath);
        psi.ArgumentList.Add(url);

        await RunProcessWithProgressAsync(psi);
    }

    // ── 2-B. 오디오(MP3) 다운로드 ───────────────────────────────────────────

    public async Task DownloadAudioAsMp3Async(string url, string formatId, string outputPath)
    {
        IsDownloading = true;
        DownloadProgress = 0;
        CurrentStatus = "다운로드 중...";
        _isAudioOnlyDownload = true;

        var psi = BuildBasePsi();

        var dir = Path.GetDirectoryName(outputPath)!;
        var nameWithoutExt = Path.GetFileNameWithoutExtension(outputPath);
        var templatePath = Path.Combine(dir, nameWithoutExt + ".%(ext)s");

        psi.ArgumentList.Add("-f");
        psi.ArgumentList.Add(formatId);
        psi.ArgumentList.Add("-x");
        psi.ArgumentList.Add("--audio-format");
        psi.ArgumentList.Add("mp3");
        psi.ArgumentList.Add("--audio-quality");
        psi.ArgumentList.Add("0");
        psi.ArgumentList.Add("--ffmpeg-location");
        psi.ArgumentList.Add(_ffmpegDir);
        psi.ArgumentList.Add("--force-overwrites");
        psi.ArgumentList.Add("--no-colors");
        psi.ArgumentList.Add("--newline");
        psi.ArgumentList.Add("-o");
        psi.ArgumentList.Add(templatePath);
        psi.ArgumentList.Add(url);

        await RunProcessWithProgressAsync(psi);
    }

    // ── 내부 헬퍼 ────────────────────────────────────────────────────────────

    // 오디오 전용 다운로드 여부 (ParseProgress 에서 변환 중 표시 여부 결정)
    private bool _isAudioOnlyDownload = false;

    // 비디오 다운로드 시 yt-dlp 는 비디오 스트림 → 오디오 스트림 순서로 받음
    // 각 스트림이 0→100% 이므로 전체 진행률로 변환:
    //   1단계(비디오): percent * 0.5          → 0~50%
    //   2단계(오디오): 50 + percent * 0.5     → 50~100%
    private int _videoDownloadStage = 0; // 0=비디오, 1=오디오

    private ProcessStartInfo BuildBasePsi() => new()
    {
        FileName = _ytdlpPath,
        RedirectStandardOutput = true,
        RedirectStandardError = true,
        UseShellExecute = false,
        CreateNoWindow = true,
        StandardOutputEncoding = System.Text.Encoding.UTF8,
        StandardErrorEncoding = System.Text.Encoding.UTF8
    };

    private async Task RunProcessWithProgressAsync(ProcessStartInfo psi)
    {
        _videoDownloadStage = 0; // 매 다운로드마다 초기화

        using var process = new Process { StartInfo = psi };
        process.Start();

        var stdoutTask = DrainStreamAsync(process.StandardOutput.BaseStream, parse: true);
        var stderrTask = DrainStreamAsync(process.StandardError.BaseStream, parse: true);

        await process.WaitForExitAsync();
        await Task.WhenAll(stdoutTask, stderrTask);

        IsDownloading = false;
        DownloadProgress = 1.0;
        CurrentStatus = process.ExitCode == 0 ? "다운로드 완료!" : "오류가 발생했습니다.";
    }

    private async Task DrainStreamAsync(Stream stream, bool parse)
    {
        var buffer = new byte[4096];
        var lineBuffer = new System.Text.StringBuilder();
        int read;

        while ((read = await stream.ReadAsync(buffer)) > 0)
        {
            var chunk = System.Text.Encoding.UTF8.GetString(buffer, 0, read);
            foreach (char ch in chunk)
            {
                if (ch == '\r' || ch == '\n')
                {
                    if (lineBuffer.Length > 0)
                    {
                        if (parse) ParseProgress(lineBuffer.ToString());
                        lineBuffer.Clear();
                    }
                }
                else
                {
                    lineBuffer.Append(ch);
                }
            }
        }

        if (parse && lineBuffer.Length > 0)
            ParseProgress(lineBuffer.ToString());
    }

    private static readonly Regex AnsiEscapeRegex =
        new(@"\x1B\[[0-9;]*[mK]", RegexOptions.Compiled);

    private void ParseProgress(string log)
    {
        log = AnsiEscapeRegex.Replace(log, "");

        // ── ffmpeg 변환 감지 (오디오 전용에만 표시) ──────────────────────
        if (log.Contains("[ExtractAudio]") ||
            log.Contains("[ffmpeg]") ||
            log.Contains("Deleting original"))
        {
            if (_isAudioOnlyDownload)
                CurrentStatus = "변환 중... (mp3)";
            return;
        }

        // ── 비디오 병합 단계 — 오버레이 표시 없이 조용히 처리 ──────────
        if (log.Contains("[Merger]") || log.Contains("Merging"))
        {
            CurrentStatus = "마무리 중...";
            return;
        }

        if (!log.Contains('%')) return;

        var match = Regex.Match(log, @"(\d+(?:\.\d+)?)%");
        if (!match.Success) return;

        if (!double.TryParse(
                match.Groups[1].Value,
                System.Globalization.NumberStyles.Any,
                System.Globalization.CultureInfo.InvariantCulture,
                out double percent)) return;

        if (percent > 100) percent = 100;

        if (_isAudioOnlyDownload)
        {
            // 오디오 전용: 그냥 0→100%
            DownloadProgress = percent / 100.0;
            CurrentStatus = $"다운로드 중... {(int)percent}%";
        }
        else
        {
            // 비디오: 1단계(비디오 스트림 0→100%) + 2단계(오디오 스트림 0→100%)
            // 1단계가 100% 찍으면 2단계로 전환
            if (_videoDownloadStage == 0 && percent >= 99.9)
                _videoDownloadStage = 1;

            double overall = _videoDownloadStage == 0
                ? percent * 0.5           // 비디오: 0~50%
                : 50 + percent * 0.5;     // 오디오: 50~100%

            DownloadProgress = overall / 100.0;

            string label = _videoDownloadStage == 0 ? "영상" : "오디오";
            CurrentStatus = $"{label} 다운로드 중... {(int)overall}%";
        }
    }

    // ── INotifyPropertyChanged ───────────────────────────────────────────────

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}