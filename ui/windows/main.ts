// Windows 화면. WinUI 3 / Fluent 2 컨트롤(InfoBar, SettingsCard, ContentDialog, ProgressRing)의 모양을 따른다.

import { h, mount } from "@shared/dom";
import { AppModel, looksLikeUrl, type DownloadState, type State } from "@shared/model";
import { formatDetail, formatDuration, formatEta, formatSize, formatSpeed, installText } from "@shared/format";
import type { FormatOption } from "@shared/types";
import { fluentIcon } from "./icons";

const model = new AppModel();
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const urlInput = $<HTMLInputElement>("url");
const urlClear = $<HTMLButtonElement>("url-clear");
const fetchButton = $<HTMLButtonElement>("fetch");
const content = $("content");
const headerItems = $("header-items");
const infobars = $("infobars");
const dialog = $("dialog");
const searchSection = $("search");
let infoBarKey = "";

$("url-icon").append(fluentIcon("link"));
urlClear.append(fluentIcon("cancel"));

// ───────── 입력 ─────────

const submit = () => void model.fetchInfo(urlInput.value);
fetchButton.addEventListener("click", submit);
urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    submit();
  } else if (e.key === "Escape") {
    clearUrl();
  }
});
urlInput.addEventListener("input", syncInput);
urlInput.addEventListener("paste", () => {
  setTimeout(() => {
    syncInput();
    if (looksLikeUrl(urlInput.value)) submit();
  });
});
urlClear.addEventListener("click", clearUrl);

function syncInput() {
  urlClear.hidden = urlInput.value.length === 0 || urlInput.disabled;
  fetchButton.disabled = urlInput.value.trim().length === 0 || urlInput.disabled;
}

function clearUrl() {
  urlInput.value = "";
  model.clearInfo();
  syncInput();
  urlInput.focus();
}

document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "v" && document.activeElement !== urlInput && !model.state.download) {
    urlInput.focus();
  }
  if (e.key === "Escape" && model.state.download) {
    const s = model.state.download.status;
    if (s === "finished" || s === "failed") model.dismissDownload();
  }
});

// ───────── 그리기 ─────────

model.subscribe((s, prev) => {
  // Windows 10 에는 Mica 가 없으므로 단색 배경을 쓴다.
  document.documentElement.classList.toggle("no-mica", s.system ? !s.system.supportsMica : false);

  renderHeader(s);
  renderInfoBars(s);

  const ready = s.phase === "ready";
  searchSection.hidden = !ready;
  urlInput.disabled = s.loadingInfo || !!s.download;
  syncInput();

  if (
    s.phase !== prev.phase ||
    s.install !== prev.install ||
    s.setupError !== prev.setupError ||
    s.loadingInfo !== prev.loadingInfo ||
    s.info !== prev.info ||
    s === prev
  ) {
    renderContent(s);
  }
  if (s.download !== prev.download || s === prev) renderDialog(s.download);
  if (prev.phase !== "ready" && ready) urlInput.focus();
});

function renderHeader(s: State) {
  const items: HTMLElement[] = [];
  if (s.updating) {
    const pct = s.install?.fraction != null ? ` ${Math.round(s.install.fraction * 100)}%` : "";
    items.push(h("span", { class: "header-status" }, h("span", { class: "ring small" }), `yt-dlp 업데이트 중${pct}`));
  }
  if (s.tools?.ytdlpVersion) {
    items.push(h("span", { class: "caption secondary", title: "yt-dlp 버전" }, `yt-dlp ${s.tools.ytdlpVersion}`));
  }
  mount(headerItems, ...items);
}

function infoBar(
  severity: "informational" | "error" | "warning",
  title: string,
  message: string,
  action?: { label: string; onClick: () => void; disabled?: boolean },
  onClose?: () => void,
) {
  const iconName = severity === "error" ? "error" : severity === "warning" ? "warning" : "info";
  return h(
    "div",
    { class: `infobar ${severity}`, role: "status" },
    h("span", { class: "infobar-icon" }, fluentIcon(iconName)),
    h("div", { class: "infobar-text" }, h("strong", null, title), " ", h("span", { class: "selectable" }, message)),
    action ? h("button", { class: "button", onclick: action.onClick, disabled: action.disabled }, action.label) : null,
    onClose ? h("button", { class: "button subtle icon-only", onclick: onClose, "aria-label": "닫기" }, fluentIcon("cancel")) : null,
  );
}

function renderInfoBars(s: State) {
  // 내용이 같으면 다시 만들지 않는다 (등장 애니메이션이 반복되지 않도록).
  const key = JSON.stringify([s.phase, s.updateError, s.tools?.updateAvailable, s.tools?.latestYtdlpVersion, s.updating, !!s.download, s.infoError]);
  if (key === infoBarKey) return;
  infoBarKey = key;
  const bars: HTMLElement[] = [];
  if (s.phase === "ready") {
    if (s.updateError) {
      bars.push(
        infoBar("warning", "yt-dlp 업데이트 실패", s.updateError, { label: "다시 시도", onClick: () => model.updateNow() }, () =>
          model.dismissUpdateError(),
        ),
      );
    } else if (s.tools?.updateAvailable && !s.updating) {
      bars.push(
        infoBar("informational", "업데이트 가능", `yt-dlp ${s.tools.latestYtdlpVersion}이(가) 나왔습니다.`, {
          label: "업데이트",
          onClick: () => model.updateNow(),
          disabled: !!s.download,
        }),
      );
    }
    if (s.infoError) bars.push(infoBar("error", "정보를 가져오지 못했습니다.", s.infoError));
  }
  mount(infobars, ...bars);
}

function renderContent(s: State) {
  switch (s.phase) {
    case "booting":
      return mount(content);
    case "installing":
      return mount(content, setupView(s));
    case "setupFailed":
      return mount(content, setupFailedView(s));
    case "ready":
      if (s.loadingInfo) return mount(content, loadingView());
      if (s.info) return mount(content, infoView(s));
      return mount(content, emptyView(s));
  }
}

function progressBar(fraction: number | null) {
  return h(
    "div",
    { class: `progressbar ${fraction == null ? "indeterminate" : ""}`, role: "progressbar" },
    h("div", { class: "progressbar-fill", style: { width: `${(fraction ?? 0) * 100}%` } }),
  );
}

function setupView(s: State) {
  const p = s.install;
  const fraction = p?.stage === "downloading" ? p.fraction : null;
  return h(
    "div",
    { class: "placeholder" },
    h("span", { class: "ring large" }),
    h("h2", { class: "subtitle" }, "처음 실행 준비 중"),
    h("p", { class: "secondary" }, installText(p)),
    h("div", { class: "setup-progress" }, progressBar(fraction)),
    h("p", { class: "caption tertiary" }, "yt-dlp와 ffmpeg를 내려받습니다. 처음 한 번만 필요합니다."),
  );
}

function setupFailedView(s: State) {
  return h(
    "div",
    { class: "placeholder" },
    fluentIcon("error", "hero error-text"),
    h("h2", { class: "subtitle" }, "필요한 구성 요소를 설치하지 못했습니다"),
    h("p", { class: "secondary selectable" }, s.setupError ?? ""),
    h("p", { class: "caption tertiary" }, "인터넷 연결을 확인한 뒤 다시 시도하세요."),
    h("button", { class: "button accent", onclick: () => model.retrySetup() }, "다시 시도"),
  );
}

function loadingView() {
  return h("div", { class: "placeholder" }, h("span", { class: "ring large" }), h("p", { class: "secondary" }, "영상 정보를 가져오는 중..."));
}

function emptyView(s: State) {
  return h(
    "div",
    { class: "placeholder" },
    fluentIcon("download", "hero tertiary"),
    h("h2", { class: "subtitle" }, "다운로드할 영상의 링크를 붙여넣으세요"),
    h("p", { class: "secondary" }, "링크를 붙여넣으면 바로 정보를 가져옵니다."),
    !s.tools?.ffmpegInstalled ? h("p", { class: "caption warning-text" }, "ffmpeg가 없어 영상과 오디오를 합칠 수 없습니다.") : null,
  );
}

function infoView(s: State) {
  const info = s.info!;
  const videos = info.formats.filter((f) => f.kind !== "audioOnly");
  const audios = info.formats.filter((f) => f.kind === "audioOnly");
  const meta = [info.uploader, formatDuration(info.duration)].filter(Boolean).join(" • ");

  return h(
    "div",
    { class: "scroll" },
    h(
      "div",
      { class: "card video" },
      h("div", { class: "thumb" }, info.thumbnail ? h("img", { src: info.thumbnail, alt: "", draggable: "false" }) : fluentIcon("play", "tertiary")),
      h("div", { class: "video-text" }, h("p", { class: "body-strong selectable" }, info.title), meta ? h("p", { class: "caption secondary" }, meta) : null),
    ),
    info.formats.length === 0 ? h("p", { class: "secondary" }, "다운로드할 수 있는 형식이 없습니다.") : null,
    videos.length ? section("동영상", videos) : null,
    audios.length ? section("오디오", audios, "오디오는 변환 없이 원본 형식 그대로 저장합니다.") : null,
  );
}

function section(title: string, formats: FormatOption[], note?: string) {
  return h(
    "section",
    { class: "section" },
    h("h3", { class: "body-strong section-title" }, title),
    formats.map(settingsCard),
    note ? h("p", { class: "caption tertiary section-note" }, note) : null,
  );
}

function settingsCard(f: FormatOption) {
  return h(
    "div",
    { class: "card settings-card" },
    fluentIcon(f.kind === "audioOnly" ? "audio" : "video", "card-icon"),
    h("div", { class: "card-text" }, h("span", { class: "body" }, f.label), h("span", { class: "caption secondary" }, formatDetail(f))),
    h("span", { class: "caption secondary size" }, formatSize(f)),
    h("button", { class: "button", onclick: () => model.download(f), title: `${f.label} 저장` }, fluentIcon("download"), "저장"),
  );
}

// ───────── ContentDialog ─────────

function renderDialog(d: DownloadState | null) {
  if (!d) {
    if (!dialog.hidden) {
      dialog.classList.add("closing");
      setTimeout(() => {
        dialog.hidden = true;
        dialog.classList.remove("closing");
        mount(dialog);
      }, 150);
    }
    return;
  }
  if (dialog.hidden) {
    // 처음 나타날 때만 등장 애니메이션을 재생한다 (진행 이벤트마다 다시 그리므로).
    dialog.classList.add("entering");
    setTimeout(() => dialog.classList.remove("entering"), 300);
  }
  dialog.hidden = false;
  dialog.classList.remove("closing");

  const done = d.status === "finished";
  const failed = d.status === "failed";
  const running = d.status === "running" || d.status === "merging";

  let title = "다운로드 중";
  let status = "";
  let detail = "";
  if (d.status === "running") {
    const part = d.stages > 1 ? (d.stage === 0 ? "영상" : "오디오") : "파일";
    status = `${part} 다운로드 중... ${Math.round(d.overall * 100)}%`;
    detail = [formatSpeed(d.speed), formatEta(d.eta)].filter(Boolean).join(" • ");
  } else if (d.status === "merging") {
    status = "영상과 오디오를 합치는 중...";
  } else if (done) {
    title = "다운로드 완료";
    status = d.path?.split(/[\\/]/).pop() ?? "";
  } else if (failed) {
    title = "다운로드 실패";
    status = d.error ?? "";
  }

  mount(
    dialog,
    h(
      "div",
      { class: "dialog", role: "dialog", "aria-modal": "true" },
      h(
        "div",
        { class: "dialog-body" },
        h("h2", { class: "subtitle dialog-title" }, done ? fluentIcon("check", "success-text") : failed ? fluentIcon("error", "error-text") : null, title),
        h("p", { class: "caption secondary ellipsis" }, `${d.title} • ${d.format.label}`),
        h("p", { class: `body status ${failed ? "selectable" : ""}` }, status),
        running ? progressBar(d.status === "merging" ? null : d.overall) : null,
        detail ? h("p", { class: "caption secondary" }, detail) : null,
      ),
      h(
        "div",
        { class: "dialog-footer" },
        done ? h("button", { class: "button accent", onclick: () => model.revealDownloaded() }, fluentIcon("folder"), "폴더에서 보기") : null,
        done || failed ? h("button", { class: "button", onclick: () => model.dismissDownload() }, "닫기") : null,
        running ? h("button", { class: "button", onclick: () => model.cancelDownload() }, "취소") : null,
      ),
    ),
  );
}

void model.start();
