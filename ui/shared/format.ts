// 표시용 문자열 도우미.

import type { FormatOption, InstallProgress } from "./types";

export function formatBytes(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return "";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
}

export function formatSize(f: FormatOption): string {
  if (f.filesize == null) return "크기 미상";
  return `${f.filesizeApprox ? "약 " : ""}${formatBytes(f.filesize)}`;
}

export function formatSpeed(bps: number | null): string {
  return bps ? `${formatBytes(bps)}/s` : "";
}

export function formatEta(sec: number | null): string {
  if (sec == null) return "";
  if (sec < 60) return `${sec}초 남음`;
  const m = Math.floor(sec / 60);
  if (m < 60) return `${m}분 ${sec % 60}초 남음`;
  return `${Math.floor(m / 60)}시간 ${m % 60}분 남음`;
}

export function formatDuration(sec: number | null): string {
  if (sec == null) return "";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${r}` : `${m}:${r}`;
}

export function formatDetail(f: FormatOption): string {
  const parts = [f.codecLabel];
  if (f.dynamicRange) parts.push(f.dynamicRange);
  if (f.kind === "muxed") parts.push("영상+오디오");
  parts.push(f.outputExt.toUpperCase());
  return parts.join(" · ");
}

export function installText(p: InstallProgress | null): string {
  if (!p) return "준비 중…";
  const name = p.tool === "ytdlp" ? "yt-dlp" : "ffmpeg";
  switch (p.stage) {
    case "downloading":
      return p.fraction != null ? `${name} 내려받는 중… ${Math.round(p.fraction * 100)}%` : `${name} 내려받는 중…`;
    case "verifying":
      return `${name} 무결성 확인 중…`;
    case "installing":
      return `${name} 설치 중…`;
    case "done":
      return `${name} 설치 완료`;
  }
}

/** Windows / macOS 에서 파일 이름에 쓸 수 없는 문자를 바꾼다. */
export function safeFileName(title: string): string {
  const cleaned = title
    .replace(/[\\/:*?"<>|\u0000-\u001f]/g, (c) => (c === ":" || c === "/" || c === "\\" || c === "|" ? "-" : c === '"' ? "'" : ""))
    .replace(/\s+/g, " ")
    .trim()
    .replace(/[. ]+$/, "");
  return (cleaned || "video").slice(0, 150);
}
