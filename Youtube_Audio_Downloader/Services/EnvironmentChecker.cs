using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace NupyYTDownloader.Services;

/// <summary>
/// %APPDATA%\NupyYTDownloader\bin\ 에서 yt-dlp.exe / ffmpeg.exe 를 관리합니다.
/// AppData 는 현재 사용자 전용 폴더이므로 관리자 권한(UAC) 없이
/// 읽기 / 쓰기 / 교체가 모두 가능합니다.
/// </summary>
public class EnvironmentChecker : INotifyPropertyChanged
{
    private const string PublisherName = "NKStudio";
    private const string AppFolderName = "YTDownloader";
    private const string GithubApiUrl = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
    private const string YtdlpDownload = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

    private bool _isYtdlpInstalled;

    public bool IsYtdlpInstalled
    {
        get => _isYtdlpInstalled;
        private set
        {
            _isYtdlpInstalled = value;
            OnPropertyChanged();
        }
    }

    private bool _isFfmpegInstalled;

    public bool IsFfmpegInstalled
    {
        get => _isFfmpegInstalled;
        private set
        {
            _isFfmpegInstalled = value;
            OnPropertyChanged();
        }
    }

    private bool _isChecking = true;

    public bool IsChecking
    {
        get => _isChecking;
        private set
        {
            _isChecking = value;
            OnPropertyChanged();
        }
    }

    private string _ytdlpVersion = "";

    public string YtdlpVersion
    {
        get => _ytdlpVersion;
        private set
        {
            _ytdlpVersion = value;
            OnPropertyChanged();
        }
    }

    private string _latestVersion = "";

    public string LatestVersion
    {
        get => _latestVersion;
        private set
        {
            _latestVersion = value;
            OnPropertyChanged();
            OnPropertyChanged(nameof(UpdateAvailable));
        }
    }

    public bool UpdateAvailable =>
        !string.IsNullOrEmpty(YtdlpVersion) &&
        !string.IsNullOrEmpty(LatestVersion) &&
        YtdlpVersion != LatestVersion;

    private bool _isUpdating;

    public bool IsUpdating
    {
        get => _isUpdating;
        private set
        {
            _isUpdating = value;
            OnPropertyChanged();
        }
    }

    private double _updateProgress;

    public double UpdateProgress
    {
        get => _updateProgress;
        private set
        {
            _updateProgress = value;
            OnPropertyChanged();
        }
    }

    public string YtdlpPath { get; private set; } = "";
    public string FfmpegPath { get; private set; } = "";
    public string BinDir { get; private set; } = "";

    public EnvironmentChecker() => CheckRequirements();

    public void CheckRequirements()
    {
        IsChecking = true;

        // ★ %APPDATA%\NupyYTDownloader\bin\ — UAC 없이 읽기/쓰기 가능
        var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        BinDir = Path.Combine(appData, PublisherName, AppFolderName, "bin");
        YtdlpPath = Path.Combine(BinDir, "yt-dlp.exe");
        FfmpegPath = Path.Combine(BinDir, "ffmpeg.exe");

        IsYtdlpInstalled = File.Exists(YtdlpPath);
        IsFfmpegInstalled = File.Exists(FfmpegPath);
        YtdlpVersion = IsYtdlpInstalled ? GetLocalVersion() : "";

        IsChecking = false;
    }

    public async Task CheckLatestVersionAsync()
    {
        try
        {
            using var client = CreateHttpClient();
            var release = await client.GetFromJsonAsync<GithubRelease>(GithubApiUrl);
            if (release is not null)
                LatestVersion = release.TagName.TrimStart('v');
        }
        catch
        {
            LatestVersion = "";
        }
    }

    public async Task<bool> UpdateYtdlpAsync()
    {
        IsUpdating = true;
        UpdateProgress = 0;

        var tempPath = YtdlpPath + ".tmp";
        try
        {
            using var client = CreateHttpClient();
            using var response = await client.GetAsync(
                YtdlpDownload, HttpCompletionOption.ResponseHeadersRead);
            response.EnsureSuccessStatusCode();

            var totalBytes = response.Content.Headers.ContentLength ?? -1L;
            await using var srcStream = await response.Content.ReadAsStreamAsync();
            await using var destStream = File.Create(tempPath);

            var buffer = new byte[81920];
            long received = 0;
            int read;

            while ((read = await srcStream.ReadAsync(buffer)) > 0)
            {
                await destStream.WriteAsync(buffer.AsMemory(0, read));
                received += read;
                if (totalBytes > 0)
                    UpdateProgress = (double)received / totalBytes;
            }

            await destStream.FlushAsync();
            destStream.Close();

            File.Move(tempPath, YtdlpPath, overwrite: true);

            YtdlpVersion = GetLocalVersion();
            LatestVersion = YtdlpVersion;
            UpdateProgress = 1.0;
            return true;
        }
        catch
        {
            if (File.Exists(tempPath))
                try
                {
                    File.Delete(tempPath);
                }
                catch
                {
                }

            return false;
        }
        finally
        {
            IsUpdating = false;
        }
    }

    private string GetLocalVersion()
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = YtdlpPath,
                Arguments = "--version",
                RedirectStandardOutput = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi);
            if (proc is null) return "";
            var ver = proc.StandardOutput.ReadLine()?.Trim() ?? "";
            proc.WaitForExit();
            return ver;
        }
        catch
        {
            return "";
        }
    }

    private static HttpClient CreateHttpClient()
    {
        var client = new HttpClient();
        client.DefaultRequestHeaders.UserAgent.ParseAdd("NupyYTDownloader/1.0");
        client.Timeout = TimeSpan.FromSeconds(30);
        return client;
    }

    private class GithubRelease
    {
        [JsonPropertyName("tag_name")] public string TagName { get; set; } = "";
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}