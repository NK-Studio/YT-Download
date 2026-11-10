// 플랫폼 공통 앱 상태와 동작. 화면(macOS / Windows)은 이 상태를 구독해 그리기만 한다.

import { backendPromise, type Backend } from "./backend";
import type { FormatOption, InstallProgress, SystemInfo, ToolStatus, VideoInfo } from "./types";
import { safeFileName } from "./format";

export type Phase = "booting" | "installing" | "setupFailed" | "ready";

export interface DownloadState {
  format: FormatOption;
  title: string;
  status: "running" | "merging" | "finished" | "cancelled" | "failed";
  overall: number;
  stage: number;
  stages: number;
  speed: number | null;
  eta: number | null;
  path: string | null;
  error: string | null;
}

export interface State {
  phase: Phase;
  system: SystemInfo | null;
  tools: ToolStatus | null;
  /** 설치/업데이트 진행 상황 (첫 설치 화면과 업데이트 표시에 함께 쓴다) */
  install: InstallProgress | null;
  setupError: string | null;
  /** 이미 설치된 상태에서 yt-dlp 를 새 버전으로 교체하는 중 */
  updating: boolean;
  updateError: string | null;

  loadingInfo: boolean;
  info: VideoInfo | null;
  infoUrl: string | null;
  infoError: string | null;

  download: DownloadState | null;
}

type Listener = (s: State, prev: State) => void;

const errorText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));

export function looksLikeUrl(text: string): boolean {
  return /^(https?:\/\/)?([\w-]+\.)*(youtube\.com|youtu\.be|youtube-nocookie\.com)\//i.test(text.trim());
}

export class AppModel {
  private backend!: Backend;
  private listeners = new Set<Listener>();
  private infoRequest = 0;

  state: State = {
    phase: "booting",
    system: null,
    tools: null,
    install: null,
    setupError: null,
    updating: false,
    updateError: null,
    loadingInfo: false,
    info: null,
    infoUrl: null,
    infoError: null,
    download: null,
  };

  get isMock() {
    return this.backend?.isMock ?? false;
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    fn(this.state, this.state);
    return () => this.listeners.delete(fn);
  }

  private set(patch: Partial<State>) {
    const prev = this.state;
    this.state = { ...prev, ...patch };
    for (const fn of this.listeners) fn(this.state, prev);
  }

  private setDownload(patch: Partial<DownloadState>) {
    if (this.state.download) this.set({ download: { ...this.state.download, ...patch } });
  }

  // ───────── 시작 / 도구 준비 ─────────

  async start() {
    this.backend = await backendPromise;
    const [system, tools] = await Promise.all([
      this.backend.systemInfo().catch(() => null),
      this.backend.toolStatus(false).catch(() => null),
    ]);
    this.set({ system, tools });

    const ready = !!tools?.ytdlpInstalled && !!tools?.ffmpegInstalled && !!tools?.ytdlpVersion;
    if (ready) {
      // 바로 쓸 수 있게 하고, 뒤에서 최신 yt-dlp 확인과 자동 업데이트를 한다.
      this.set({ phase: "ready" });
      void this.prepare(true);
    } else {
      this.set({ phase: "installing" });
      await this.prepare(false);
    }
  }

  private async prepare(background: boolean) {
    this.set({ setupError: null, install: null });
    try {
      const tools = await this.backend.prepareTools(true, (p) => {
        // 이미 설치된 상태에서 진행 이벤트가 온다 = 자동 업데이트 중
        this.set({ install: p, ...(background ? { updating: p.stage !== "done" } : {}) });
      });
      this.set({ tools, phase: "ready", install: null, updating: false });
    } catch (e) {
      if (background) {
        // 기존 바이너리로 계속 쓸 수 있으므로 화면을 막지 않는다.
        this.set({ updating: false, install: null, updateError: errorText(e) });
      } else {
        this.set({ phase: "setupFailed", setupError: errorText(e), install: null });
      }
    }
  }

  retrySetup() {
    this.set({ phase: "installing" });
    void this.prepare(false);
  }

  async updateNow() {
    if (this.state.updating || this.state.download) return;
    this.set({ updating: true, updateError: null, install: null });
    try {
      const tools = await this.backend.updateYtdlp((p) => this.set({ install: p }));
      this.set({ tools, updating: false, install: null });
    } catch (e) {
      this.set({ updating: false, install: null, updateError: errorText(e) });
    }
  }

  dismissUpdateError() {
    this.set({ updateError: null });
  }

  // ───────── 영상 정보 ─────────

  async fetchInfo(url: string) {
    url = url.trim();
    if (!url || this.state.download) return;
    if (url === this.state.infoUrl && (this.state.loadingInfo || this.state.info)) return;
    const req = ++this.infoRequest;
    this.set({ loadingInfo: true, infoError: null, info: null, infoUrl: url });
    try {
      const info = await this.backend.fetchInfo(url);
      if (req === this.infoRequest) this.set({ info, loadingInfo: false });
    } catch (e) {
      if (req === this.infoRequest) this.set({ infoError: errorText(e), loadingInfo: false, infoUrl: null });
    }
  }

  clearInfo() {
    this.infoRequest++;
    this.set({ info: null, infoUrl: null, infoError: null, loadingInfo: false });
  }

  // ───────── 다운로드 ─────────

  async download(format: FormatOption) {
    const info = this.state.info;
    const url = this.state.infoUrl;
    if (!info || !url || this.state.download) return;

    const name = `${safeFileName(info.title)}.${format.outputExt}`;
    const output = await this.backend.saveDialog(name, format.outputExt);
    if (!output) return;

    this.set({
      download: {
        format,
        title: info.title,
        status: "running",
        overall: 0,
        stage: 0,
        stages: format.kind === "videoOnly" ? 2 : 1,
        speed: null,
        eta: null,
        path: null,
        error: null,
      },
    });

    try {
      const outcome = await this.backend.startDownload(
        { url, formatId: format.id, kind: format.kind, output },
        (ev) => {
          switch (ev.type) {
            case "started":
              this.setDownload({ stages: ev.stages });
              break;
            case "progress":
              this.setDownload({ overall: ev.overall, stage: ev.stage, stages: ev.stages, speed: ev.speed, eta: ev.eta });
              break;
            case "merging":
              this.setDownload({ status: "merging", overall: 1, speed: null, eta: null });
              break;
            case "finished":
              this.setDownload({ path: ev.path });
              break;
          }
        },
      );
      if (outcome.status === "finished") {
        this.setDownload({ status: "finished", overall: 1, path: outcome.path });
      } else {
        this.set({ download: null });
      }
    } catch (e) {
      this.setDownload({ status: "failed", error: errorText(e) });
    }
  }

  cancelDownload() {
    if (this.state.download?.status === "running" || this.state.download?.status === "merging") {
      void this.backend.cancelDownload();
    }
  }

  dismissDownload() {
    const d = this.state.download;
    if (d && (d.status === "finished" || d.status === "failed")) this.set({ download: null });
  }

  revealDownloaded() {
    const path = this.state.download?.path;
    if (path) void this.backend.reveal(path);
  }
}
