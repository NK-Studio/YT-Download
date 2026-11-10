//! 자식 프로세스 생성과 강제 종료 도우미.

use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// 콘솔 창 없이, stdin 없이 실행되는 명령을 만든다.
pub fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    // yt-dlp(Python)가 콘솔 인코딩과 상관없이 UTF-8로 출력하게 한다.
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        // yt-dlp가 띄운 ffmpeg까지 한 번에 종료할 수 있도록 별도 프로세스 그룹으로 만든다.
        cmd.process_group(0);
    }
    cmd
}

/// 프로세스와 그 자식(ffmpeg, PyInstaller 부트로더의 자식 등)을 모두 종료한다.
pub fn kill_tree(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}
