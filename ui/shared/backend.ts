// Rust 명령 호출부. Tauri 밖(일반 브라우저)에서 열면 화면 확인용 목업을 쓴다.

import type {
  DownloadEvent,
  DownloadOutcome,
  DownloadRequest,
  FormatOption,
  InstallProgress,
  SystemInfo,
  ToolStatus,
  VideoInfo,
} from "./types";

export interface Backend {
  readonly isMock: boolean;
  systemInfo(): Promise<SystemInfo>;
  toolStatus(checkLatest: boolean): Promise<ToolStatus>;
  prepareTools(autoUpdate: boolean, onProgress: (p: InstallProgress) => void): Promise<ToolStatus>;
  updateYtdlp(onProgress: (p: InstallProgress) => void): Promise<ToolStatus>;
  fetchInfo(url: string): Promise<VideoInfo>;
  startDownload(req: DownloadRequest, onEvent: (e: DownloadEvent) => void): Promise<DownloadOutcome>;
  cancelDownload(): Promise<void>;
  /** 네이티브 저장 대화상자. 취소하면 null. */
  saveDialog(defaultName: string, ext: string): Promise<string | null>;
  /** Finder / 탐색기에서 파일 위치 열기 */
  reveal(path: string): Promise<void>;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriBackend(): Promise<Backend> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const { downloadDir, join } = await import("@tauri-apps/api/path");
  const { save } = await import("@tauri-apps/plugin-dialog");
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");

  const channel = <T>(fn: (v: T) => void) => {
    const ch = new Channel<T>();
    ch.onmessage = fn;
    return ch;
  };

  return {
    isMock: false,
    systemInfo: () => invoke("system_info"),
    toolStatus: (checkLatest) => invoke("tool_status", { checkLatest }),
    prepareTools: (autoUpdate, onProgress) =>
      invoke("prepare_tools", { autoUpdate, onProgress: channel(onProgress) }),
    updateYtdlp: (onProgress) => invoke("update_ytdlp", { onProgress: channel(onProgress) }),
    fetchInfo: (url) => invoke("fetch_info", { url }),
    startDownload: (request, onEvent) => invoke("start_download", { request, onEvent: channel(onEvent) }),
    cancelDownload: () => invoke("cancel_download"),
    async saveDialog(defaultName, ext) {
      let defaultPath = defaultName;
      try {
        defaultPath = await join(await downloadDir(), defaultName);
      } catch {
        /* 다운로드 폴더를 모르면 이름만 넘긴다 */
      }
      const label = ext === "mp4" ? "MPEG-4 동영상" : `${ext.toUpperCase()} 파일`;
      return save({ title: "저장할 위치를 선택하세요", defaultPath, filters: [{ name: label, extensions: [ext] }] });
    },
    reveal: (path) => revealItemInDir(path),
  };
}

// ───────────────────────── 목업 ─────────────────────────

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function mockBackend(): Backend {
  // 일반 브라우저 미리보기: 창 배경 효과(vibrancy/Mica)가 없으므로 단색 배경을 쓰게 한다.
  document.documentElement.classList.add("preview");
  const params = new URLSearchParams(location.search);
  let installed = params.get("fresh") === null;
  let cancelled = false;
  const status = (): ToolStatus => ({
    ytdlpInstalled: installed,
    ytdlpVersion: installed ? "2026.09.30" : null,
    latestYtdlpVersion: "2026.10.26",
    ffmpegInstalled: installed,
    updateAvailable: installed,
  });
  const fmt = (o: Partial<FormatOption> & Pick<FormatOption, "id" | "kind" | "label" | "codecLabel">): FormatOption => ({
    ext: "mp4",
    outputExt: o.kind === "audioOnly" ? "m4a" : "mp4",
    height: null,
    fps: null,
    abr: null,
    vcodec: null,
    acodec: null,
    filesize: null,
    filesizeApprox: false,
    dynamicRange: null,
    ...o,
  });
  const fakeInstall = async (tool: "ytdlp" | "ffmpeg", on: (p: InstallProgress) => void) => {
    for (let i = 0; i <= 20; i++) {
      on({ tool, stage: "downloading", fraction: i / 20 });
      await sleep(60);
    }
    on({ tool, stage: "verifying", fraction: null });
    await sleep(300);
    on({ tool, stage: "installing", fraction: null });
    await sleep(300);
    on({ tool, stage: "done", fraction: 1 });
  };

  return {
    isMock: true,
    systemInfo: async () => ({ appVersion: "2.0.0", supportsMica: true }),
    toolStatus: async () => status(),
    async prepareTools(_autoUpdate, on) {
      if (!installed) {
        await fakeInstall("ytdlp", on);
        await fakeInstall("ffmpeg", on);
        installed = true;
        return { ...status(), updateAvailable: false, ytdlpVersion: "2026.10.26" };
      }
      return status();
    },
    async updateYtdlp(on) {
      await fakeInstall("ytdlp", on);
      return { ...status(), ytdlpVersion: "2026.10.26", updateAvailable: false };
    },
    async fetchInfo(url) {
      await sleep(900);
      if (url.includes("fail")) throw "[youtube] fail: Video unavailable. This video is private";
      return {
        id: "jNQXAC9IVRw",
        title: "Me at the zoo — 동물원에서 찍은 첫 번째 영상",
        uploader: "jawed",
        duration: 19,
        thumbnail: "",
        webpageUrl: url,
        formats: [
          fmt({ id: "401", kind: "videoOnly", label: "2160p", codecLabel: "AV1", height: 2160, filesize: 812_000_000, dynamicRange: "HDR10" }),
          fmt({ id: "137", kind: "videoOnly", label: "1080p60", codecLabel: "H.264", height: 1080, fps: 60, filesize: 143_200_000 }),
          fmt({ id: "248", kind: "videoOnly", label: "1080p", codecLabel: "VP9", height: 1080, filesize: 98_500_000, filesizeApprox: true }),
          fmt({ id: "22", kind: "muxed", label: "720p", codecLabel: "H.264", height: 720, filesize: 61_000_000 }),
          fmt({ id: "18", kind: "muxed", label: "360p", codecLabel: "H.264", height: 360, filesize: 18_000_000 }),
          fmt({ id: "140", kind: "audioOnly", ext: "m4a", label: "130 kbps", codecLabel: "AAC", abr: 130, filesize: 3_100_000 }),
          fmt({ id: "251", kind: "audioOnly", ext: "webm", outputExt: "webm", label: "106 kbps", codecLabel: "Opus", abr: 106, filesize: 2_500_000 }),
        ],
      };
    },
    async startDownload(req, on) {
      cancelled = false;
      const stages = req.kind === "videoOnly" ? 2 : 1;
      on({ type: "started", stages });
      for (let i = 0; i <= 100; i += 2) {
        if (cancelled) return { status: "cancelled" };
        const stage = stages === 2 && i >= 50 ? 1 : 0;
        on({ type: "progress", stage, stages, overall: i / 100, downloadedBytes: i * 1_200_000, totalBytes: 120_000_000, speed: 8_400_000, eta: Math.round((100 - i) / 5) });
        await sleep(120);
      }
      if (stages === 2) {
        on({ type: "merging" });
        await sleep(800);
      }
      on({ type: "finished", path: req.output });
      return { status: "finished", path: req.output };
    },
    async cancelDownload() {
      cancelled = true;
    },
    saveDialog: async (name) => `/Users/me/Downloads/${name}`,
    reveal: async () => {},
  };
}

export const backendPromise: Promise<Backend> = inTauri ? tauriBackend() : Promise.resolve(mockBackend());
