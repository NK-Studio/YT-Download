using Microsoft.UI.Xaml;
using Microsoft.UI;
using System;
using System.Diagnostics;
using Windows.Graphics;
using Windows.UI;
using Windows.UI.ViewManagement;
using System.IO;
using Microsoft.UI.Windowing;

namespace NupyYTDownloader;

public partial class App : Application
{
    private Window? _window;
    private ApplicationTheme? _lastTheme;
    private UISettings? _uiSettings;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Step 1: 간단하게 PrinterDiscoveryWindow만 표시
        _uiSettings = new UISettings();
        _window = new MainWindow();
        InitializeWindowSize(_window);

        SetTitleBarColors(_window);
        UpdateIcon(_window);

        _uiSettings.ColorValuesChanged += UiSettings_ColorValuesChanged;

        _window.Closed += (_, _) =>
        {
            if (_uiSettings != null)
                _uiSettings.ColorValuesChanged -= UiSettings_ColorValuesChanged;
        };

        _window.Activate();
    }

    private void InitializeWindowSize(Window window) // ← 매개변수 추가
    {
        try
        {
            // ✅ window 매개변수 사용
            IntPtr hWnd = WinRT.Interop.WindowNative.GetWindowHandle(window);
            WindowId wndId = Win32Interop.GetWindowIdFromWindow(hWnd);

            AppWindow? appWindow = AppWindow.GetFromWindowId(wndId);
            if (appWindow == null)
            {
                Debug.WriteLine("AppWindow를 가져올 수 없습니다.");
                return;
            }

            DisplayArea? displayArea = DisplayArea.GetFromWindowId(wndId, DisplayAreaFallback.Primary);
            if (displayArea == null)
            {
                Debug.WriteLine("DisplayArea를 가져올 수 없습니다.");
                return;
            }

            int screenHeight = displayArea.WorkArea.Height;
            int screenWidth = displayArea.WorkArea.Width;

            // 목표 비율 (960:1400)
            double targetAspectRatio = 960.0 / 1400.0;

            // 높이 계산 (화면의 85%, 최대 1400)
            int newHeight = Math.Min((int)(screenHeight * 0.85), 1400);

            // 비율에 맞춰 너비 계산
            int newWidth = (int)(newHeight * targetAspectRatio);

            // 크기 적용
            appWindow.Resize(new SizeInt32(newWidth, newHeight));

            // 중앙 정렬
            int x = (screenWidth - newWidth) / 2;
            int y = (screenHeight - newHeight) / 2;
            appWindow.Move(new PointInt32(displayArea.WorkArea.X + x, displayArea.WorkArea.Y + y));
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"창 크기 초기화 실패: {ex.Message}");
        }
    }

    private AppWindow? GetAppWindowForWindow(Window window)
    {
        var windowHandle = WinRT.Interop.WindowNative.GetWindowHandle(window);
        var windowId = Win32Interop.GetWindowIdFromWindow(windowHandle);
        return AppWindow.GetFromWindowId(windowId);
    }

    private void UiSettings_ColorValuesChanged(UISettings sender, object args)
    {
        if (_window?.DispatcherQueue == null)
            return;

        _window.DispatcherQueue.TryEnqueue(() =>
        {
            var currentTheme = Current.RequestedTheme;

            // 테마가 실제로 변경되었을 때만 처리
            if (_lastTheme == currentTheme)
                return;

            _lastTheme = currentTheme;

            SetTitleBarColors(_window);
        });
    }

    private void SetTitleBarColors(Window window)
    {
        var appWindow = GetAppWindowForWindow(window);

        // 커스터마이징 지원 여부 확인
        if (AppWindowTitleBar.IsCustomizationSupported())
        {
            if (appWindow != null)
            {
                var titleBar = appWindow.TitleBar;

                // 1. 현재 테마가 다크 모드인지 확인
                bool isDark = Current.RequestedTheme == ApplicationTheme.Dark;

                // 2. 테마별 색상 정의
                // [배경색]
                // 다크: 진한 회색 (32,32,32), 라이트: 흰색
                var bgColor = isDark
                    ? Color.FromArgb(255, 32, 32, 32)
                    : Colors.White;

                // 비활성 배경 (다크: 더 어둡게, 라이트: 아주 연한 회색)
                var inactiveBgColor = isDark
                    ? Color.FromArgb(255, 20, 20, 20)
                    : Color.FromArgb(255, 243, 243, 243);

                // [전경색(글자/아이콘)]
                // 다크: 흰색, 라이트: 검은색
                var fgColor = isDark ? Colors.White : Colors.Black;
                // 비활성 글자는 둘 다 회색으로 처리
                var inactiveFgColor = Colors.Gray;

                // [버튼 상호작용 색상]
                // 마우스 올렸을 때 (Hover)
                var hoverBgColor = isDark
                    ? Color.FromArgb(255, 60, 60, 60)
                    : Color.FromArgb(255, 230, 230, 230);

                // 눌렀을 때 (Pressed)
                var pressedBgColor = isDark
                    ? Color.FromArgb(255, 80, 80, 80)
                    : Color.FromArgb(255, 210, 210, 210);

                // 3. 타이틀 바 전체 색상 적용
                titleBar.ForegroundColor = fgColor;
                titleBar.BackgroundColor = bgColor;

                // 4. 버튼 색상 적용 (Transparent 대신 배경색과 동일하게 설정)
                // 평상시 버튼 배경 = 타이틀바 배경
                titleBar.ButtonBackgroundColor = bgColor;
                titleBar.ButtonForegroundColor = fgColor;

                // 마우스 올렸을 때 (Hover)
                titleBar.ButtonHoverBackgroundColor = hoverBgColor;
                titleBar.ButtonHoverForegroundColor = fgColor;

                // 눌렀을 때 (Pressed)
                titleBar.ButtonPressedBackgroundColor = pressedBgColor;
                titleBar.ButtonPressedForegroundColor = fgColor;

                // 5. 비활성 상태(Inactive) 색상 적용
                titleBar.InactiveForegroundColor = inactiveFgColor;
                titleBar.InactiveBackgroundColor = inactiveBgColor;

                // 비활성 버튼 배경 = 비활성 타이틀바 배경
                titleBar.ButtonInactiveBackgroundColor = inactiveBgColor;
                titleBar.ButtonInactiveForegroundColor = inactiveFgColor;
            }
        }
    }

    private void UpdateIcon(Window window)
    {
        var appWindow = GetAppWindowForWindow(window);

        if (appWindow != null)
        {
            // 패키징 여부와 상관없이, 실행 파일이 있는 폴더 기준으로 경로를 잡습니다.
            string iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", "icon.ico");

            // 파일이 실제로 존재하는지 확인 (디버깅용)
            if (File.Exists(iconPath))
            {
                appWindow.SetIcon(iconPath);
            }
        }
    }
}
