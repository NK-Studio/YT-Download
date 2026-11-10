// Rust(nupy-core / src-tauri)에서 넘어오는 값의 타입. serde camelCase 와 맞춘다.

export interface ToolStatus {
  ytdlpInstalled: boolean;
  ytdlpVersion: string | null;
  latestYtdlpVersion: string | null;
  ffmpegInstalled: boolean;
  updateAvailable: boolean;
}

export type Tool = "ytdlp" | "ffmpeg";
export type InstallStage = "downloading" | "verifying" | "installing" | "done";

export interface InstallProgress {
  tool: Tool;
  stage: InstallStage;
  fraction: number | null;
}

export type FormatKind = "muxed" | "videoOnly" | "audioOnly";

export interface FormatOption {
  id: string;
  kind: FormatKind;
  ext: string;
  outputExt: string;
  label: string;
  codecLabel: string;
  height: number | null;
  fps: number | null;
  abr: number | null;
  vcodec: string | null;
  acodec: string | null;
  filesize: number | null;
  filesizeApprox: boolean;
  dynamicRange: string | null;
}

export interface VideoInfo {
  id: string;
  title: string;
  uploader: string | null;
  duration: number | null;
  thumbnail: string | null;
  webpageUrl: string | null;
  formats: FormatOption[];
}

export interface DownloadRequest {
  url: string;
  formatId: string;
  kind: FormatKind;
  output: string;
}

export type DownloadEvent =
  | { type: "started"; stages: number }
  | {
      type: "progress";
      stage: number;
      stages: number;
      overall: number;
      downloadedBytes: number | null;
      totalBytes: number | null;
      speed: number | null;
      eta: number | null;
    }
  | { type: "merging" }
  | { type: "finished"; path: string };

export type DownloadOutcome = { status: "finished"; path: string } | { status: "cancelled" };

export interface SystemInfo {
  appVersion: string;
  supportsMica: boolean;
}
