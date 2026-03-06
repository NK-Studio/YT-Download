using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Graphics;
using Windows.Storage.Pickers;
using Windows.System;
using WinRT.Interop;
using NupyYTDownloader.Models;
using NupyYTDownloader.Services;

namespace NupyYTDownloader;

public sealed partial class MainWindow : Window
{
    // ── 서비스 ───────────────────────────────────────────────────────────────

    private readonly EnvironmentChecker _env;
    private YoutubeDownloader? _downloader;

    // ── 현재 영상 정보 ────────────────────────────────────────────────────────

    private VideoMetadata? _currentMetadata;
    private List<VideoFormat> _currentFormats = new();

    // ── 생성자 ────────────────────────────────────────────────────────────────

    public MainWindow()
    {
        this.InitializeComponent();

        SetWindowSize(480, 720);
        TryEnableMica();

        // 환경 확인
        _env = new EnvironmentChecker();
        _env.PropertyChanged += OnEnvPropertyChanged;

        if (_env.IsYtdlpInstalled && _env.IsFfmpegInstalled)
        {
            _downloader = new YoutubeDownloader(_env.YtdlpPath, _env.FfmpegPath);
            _downloader.PropertyChanged += OnDownloaderPropertyChanged;

            // 버전 뱃지 표시
            if (!string.IsNullOrEmpty(_env.YtdlpVersion))
            {
                VersionText.Text = _env.YtdlpVersion;
                VersionBadge.Visibility = Visibility.Visible;
            }

            // ★ GitHub 최신 버전 비동기 체크 (UI 블로킹 없음)
            _ = _env.CheckLatestVersionAsync();
        }
        else
        {
            StatusDot.Fill = new SolidColorBrush(Colors.Red);
            ErrorInstallOverlay.Visibility = Visibility.Visible;
        }
    }

    // ── 윈도우 크기 / 소재 설정 ──────────────────────────────────────────────

    private void SetWindowSize(int width, int height)
    {
        var hwnd = WindowNative.GetWindowHandle(this);
        var windowId = Win32Interop.GetWindowIdFromWindow(hwnd);
        var appWindow = AppWindow.GetFromWindowId(windowId);
        appWindow.Resize(new SizeInt32(width, height));
        appWindow.Title = "YT Downloader";
    }

    private void TryEnableMica()
    {
        try
        {
            // Windows 11 에서 Mica 소재 적용 (Windows 10 에서는 무시됨)
            SystemBackdrop = new Microsoft.UI.Xaml.Media.MicaBackdrop();
        }
        catch { /* 지원하지 않는 OS 버전 — 무시 */ }
    }

    // ── Downloader PropertyChanged → UI 갱신 ─────────────────────────────────

    private void OnDownloaderPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            if (_downloader is null) return;

            switch (e.PropertyName)
            {
                case nameof(YoutubeDownloader.IsDownloading):
                    ProgressOverlay.Visibility = _downloader.IsDownloading
                        ? Visibility.Visible : Visibility.Collapsed;

                    if (_downloader.IsDownloading)
                    {
                        DoneButton.Visibility = Visibility.Collapsed;
                        DownloadRing.IsActive = true;
                        DownloadProgressBar.Value = 0;
                    }
                    break;

                case nameof(YoutubeDownloader.DownloadProgress):
                    DownloadProgressBar.Value = _downloader.DownloadProgress;

                    if (_downloader.DownloadProgress >= 1.0)
                        DownloadRing.IsActive = false;
                    break;

                case nameof(YoutubeDownloader.CurrentStatus):
                    StatusText.Text = _downloader.CurrentStatus;

                    if (_downloader.CurrentStatus.Contains("변환 중"))
                    {
                        // ffmpeg MP3 변환 중 — Indeterminate
                        DownloadProgressBar.IsIndeterminate = true;
                        DownloadRing.IsActive = true;
                    }
                    else if (_downloader.CurrentStatus.Contains("마무리 중"))
                    {
                        // 영상+오디오 병합 중 — Indeterminate (짧게 스침)
                        DownloadProgressBar.IsIndeterminate = true;
                        DownloadRing.IsActive = true;
                    }
                    else if (_downloader.CurrentStatus.Contains("완료"))
                    {
                        DownloadProgressBar.IsIndeterminate = false;
                        DownloadProgressBar.Value = 1.0;
                        DownloadRing.IsActive = false;
                        DoneButton.Visibility = Visibility.Visible;
                    }
                    break;
            }
        });
    }

    // ── EnvironmentChecker PropertyChanged → 업데이트 버튼 UI 갱신 ───────────

    private void OnEnvPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            switch (e.PropertyName)
            {
                // GitHub 버전 조회 완료 → 업데이트 버튼 표시 여부 결정
                case nameof(EnvironmentChecker.UpdateAvailable):
                case nameof(EnvironmentChecker.LatestVersion):
                    UpdateButton.Visibility = _env.UpdateAvailable
                        ? Visibility.Visible : Visibility.Collapsed;
                    break;

                // 업데이트 다운로드 중 → 버튼 상태 변경
                case nameof(EnvironmentChecker.IsUpdating):
                    UpdateButton.IsEnabled = !_env.IsUpdating;
                    UpdateRing.IsActive = _env.IsUpdating;
                    UpdateRing.Visibility = _env.IsUpdating ? Visibility.Visible : Visibility.Collapsed;
                    UpdateIcon.Visibility = _env.IsUpdating ? Visibility.Collapsed : Visibility.Visible;
                    UpdateButtonText.Text = _env.IsUpdating ? "업데이트 중..." : "업데이트";
                    break;

                // 다운로드 진행률 → 버튼 텍스트에 퍼센트 표시
                case nameof(EnvironmentChecker.UpdateProgress):
                    if (_env.IsUpdating)
                        UpdateButtonText.Text = $"{(int)(_env.UpdateProgress * 100)}%";
                    break;

                // 업데이트 완료 → 버전 뱃지 갱신
                case nameof(EnvironmentChecker.YtdlpVersion):
                    if (!string.IsNullOrEmpty(_env.YtdlpVersion))
                    {
                        VersionText.Text = _env.YtdlpVersion;
                        VersionBadge.Visibility = Visibility.Visible;
                    }
                    break;
            }
        });
    }

    // ── 업데이트 버튼 클릭 ────────────────────────────────────────────────────

    private async void UpdateButton_Click(object sender, RoutedEventArgs e)
    {
        var success = await _env.UpdateYtdlpAsync();

        DispatcherQueue.TryEnqueue(() =>
        {
            if (success)
            {
                // 업데이트 성공 → 버튼 숨기고 버전 뱃지 갱신
                UpdateButton.Visibility = Visibility.Collapsed;
                UpdateButtonText.Text = "업데이트";
                UpdateIcon.Visibility = Visibility.Visible;
                UpdateButton.IsEnabled = true;
            }
            else
            {
                // 실패 → 버튼 원상복구
                UpdateButtonText.Text = "재시도";
                UpdateButton.IsEnabled = true;
            }
        });
    }

    // ── URL 입력 Enter 키 처리 ───────────────────────────────────────────────

    private async void UrlTextBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Enter)
        {
            e.Handled = true;
            await FetchFormatsAsync();
        }
    }

    // ── 포맷 목록 가져오기 ────────────────────────────────────────────────────

    private async Task FetchFormatsAsync()
    {
        var url = UrlTextBox.Text.Trim();
        if (string.IsNullOrEmpty(url) || _downloader is null) return;

        // UI 초기화
        ErrorText.Visibility = Visibility.Collapsed;
        ThumbnailSection.Visibility = Visibility.Collapsed;
        FormatsPanel.Children.Clear();
        _currentFormats.Clear();

        ShowLoading(true);

        try
        {
            var (metadata, formats) = await _downloader.FetchVideoInfoAsync(url);
            _currentMetadata = metadata;
            _currentFormats = formats;

            // 썸네일 + 제목 표시
            ThumbnailSection.Visibility = Visibility.Visible;
            VideoTitleText.Text = metadata.Title;
            ThumbnailImage.Source = new BitmapImage(new Uri(metadata.ThumbnailUrl));

            // 포맷 카드 생성
            if (formats.Count == 0)
            {
                FormatsPanel.Children.Add(new TextBlock
                {
                    Text = "다운로드 가능한 포맷이 없습니다.",
                    Foreground = new SolidColorBrush(Colors.Gray),
                    Margin = new Thickness(0, 8, 0, 0)
                });
            }
            else
            {
                foreach (var fmt in formats)
                    FormatsPanel.Children.Add(BuildFormatCard(fmt));
            }
        }
        catch (Exception ex)
        {
            ErrorText.Text = $"정보를 가져오는데 실패했습니다: {ex.Message}";
            ErrorText.Visibility = Visibility.Visible;
        }
        finally
        {
            ShowLoading(false);
        }
    }

    private void ShowLoading(bool visible)
    {
        LoadingPanel.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
        FormatLoadingRing.IsActive = visible;
        HeaderLoadingRing.IsActive = visible;
        HeaderLoadingRing.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
        UrlTextBox.IsEnabled = !visible;
    }

    // ── 포맷 카드 UI 생성 ─────────────────────────────────────────────────────

    private Button BuildFormatCard(VideoFormat format)
    {
        var accentColor = Windows.UI.Color.FromArgb(255, 0, 183, 255); // #00B7FF

        // 왼쪽: 해상도 + 코덱 정보
        var leftStack = new StackPanel { Spacing = 2 };
        leftStack.Children.Add(new TextBlock
        {
            Text = format.Resolution,
            FontSize = 16,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold
        });
        leftStack.Children.Add(new TextBlock
        {
            Text = $"{format.DisplayExt}  •  {format.CodecTag}",
            FontSize = 11,
            Foreground = new SolidColorBrush(Colors.Gray)
        });

        // 오른쪽: 파일 크기 + 다운로드 아이콘
        var rightStack = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            VerticalAlignment = VerticalAlignment.Center
        };
        rightStack.Children.Add(new TextBlock
        {
            Text = format.FileSize,
            Foreground = new SolidColorBrush(accentColor),
            FontSize = 13,
            VerticalAlignment = VerticalAlignment.Center
        });
        rightStack.Children.Add(new FontIcon
        {
            Glyph = "\uE896", // Download icon
            FontSize = 20,
            Foreground = new SolidColorBrush(accentColor)
        });

        // 카드 그리드
        var grid = new Grid();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        Grid.SetColumn(leftStack, 0);
        Grid.SetColumn(rightStack, 1);
        grid.Children.Add(leftStack);
        grid.Children.Add(rightStack);

        var button = new Button
        {
            Content = grid,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(14, 12, 14, 12),
            CornerRadius = new CornerRadius(10)
        };

        button.Click += async (_, _) => await ShowSaveDialogAsync(format);
        return button;
    }

    // ── 저장 대화상자 & 다운로드 시작 ────────────────────────────────────────

    private async Task ShowSaveDialogAsync(VideoFormat format)
    {
        if (_downloader is null) return;

        var hwnd = WindowNative.GetWindowHandle(this);

        var picker = new FileSavePicker();
        InitializeWithWindow.Initialize(picker, hwnd);

        picker.SuggestedStartLocation = PickerLocationId.Downloads;

        string ext = format.DisplayExt; // mp3 또는 mp4
        picker.FileTypeChoices.Add(ext.ToUpper() + " 파일", new List<string> { $".{ext}" });

        if (_currentMetadata is not null)
            picker.SuggestedFileName = $"{SanitizeFileName(_currentMetadata.Title)}.{ext}";

        var file = await picker.PickSaveFileAsync();
        if (file is null) return;

        var savePath = file.Path;

        // ★ 핵심 수정:
        // FileSavePicker는 사용자가 저장을 누르는 순간 빈 파일(0KB)을 생성합니다.
        // yt-dlp는 파일이 이미 존재하면 "이미 다운로드됨"으로 판단하고 건너뜁니다.
        // → 빈 파일을 즉시 삭제해서 yt-dlp가 새로 만들도록 합니다.
        try
        {
            if (System.IO.File.Exists(savePath))
                System.IO.File.Delete(savePath);
        }
        catch { /* 삭제 실패해도 --force-overwrites 로 커버 */ }

        if (format.IsAudioOnly)
            await _downloader.DownloadAudioAsMp3Async(UrlTextBox.Text.Trim(), format.Id, savePath);
        else
            await _downloader.DownloadVideoAsync(UrlTextBox.Text.Trim(), format.Id, savePath);
    }

    // ── 버튼 이벤트 핸들러 ────────────────────────────────────────────────────

    private void DoneButton_Click(object sender, RoutedEventArgs e)
    {
        ProgressOverlay.Visibility = Visibility.Collapsed;
        DoneButton.Visibility = Visibility.Collapsed;
    }

    private void RetryButton_Click(object sender, RoutedEventArgs e)
    {
        _env.CheckRequirements();

        if (_env.IsYtdlpInstalled && _env.IsFfmpegInstalled)
        {
            ErrorInstallOverlay.Visibility = Visibility.Collapsed;
            StatusDot.Fill = new SolidColorBrush(Colors.Green);

            if (!string.IsNullOrEmpty(_env.YtdlpVersion))
            {
                VersionText.Text = _env.YtdlpVersion;
                VersionBadge.Visibility = Visibility.Visible;
            }

            _downloader = new YoutubeDownloader(_env.YtdlpPath, _env.FfmpegPath);
            _downloader.PropertyChanged += OnDownloaderPropertyChanged;

            // GitHub 버전 체크도 다시 실행
            _ = _env.CheckLatestVersionAsync();
        }
        else
        {
            StatusDot.Fill = new SolidColorBrush(Colors.Red);
            VersionBadge.Visibility = Visibility.Collapsed;
        }
    }

    // ── 유틸리티 ──────────────────────────────────────────────────────────────

    /// <summary>Windows 파일명으로 사용할 수 없는 문자를 제거/치환합니다.</summary>
    private static string SanitizeFileName(string title) =>
        title
            .Replace("/", "-")
            .Replace("\\", "-")
            .Replace(":", "-")
            .Replace("*", "")
            .Replace("?", "")
            .Replace("\"", "'")
            .Replace("<", "")
            .Replace(">", "")
            .Replace("|", "-");
}