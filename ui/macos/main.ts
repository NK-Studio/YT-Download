// macOS 화면. AppKit / SwiftUI 의 모양과 동작(시트, 인셋 그룹 목록, 툴바)을 따른다.

import { h, mount } from "@shared/dom";
import { AppModel, looksLikeUrl, type DownloadState, type State } from "@shared/model";
import { formatDetail, formatDuration, formatEta, formatSize, formatSpeed, installText } from "@shared/format";
import type { FormatOption } from "@shared/types";
import { icons } from "./icons";

const model = new AppModel();
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const urlInput = $<HTMLInputElement>("url");
const urlClear = $<HTMLButtonElement>("url-clear");
const urlError = $<HTMLParagraphElement>("url-error");
const content = $("content");
const toolbarItems = $("toolbar-items");
const sheet = $("sheet");
const searchSection = $("search");

$("field-icon").innerHTML = icons.link;
urlClear.innerHTML = icons.clear;

const icon = (svg: string, cls = "icon") => h("span", { class: cls, html: svg });

// ───────── 입력 ─────────

urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    void model.fetchInfo(urlInput.value);
  } else if (e.key === "Escape") {
    clearUrl();
  }
});
urlInput.addEventListener("input", () => {
  urlClear.hidden = urlInput.value.length === 0;
});
// 링크를 붙여넣으면 바로 조회한다.
urlInput.addEventListener("paste", () => {
  setTimeout(() => {
    urlClear.hidden = urlInput.value.length === 0;
    if (looksLikeUrl(urlInput.value)) void model.fetchInfo(urlInput.value);
  });
});
urlClear.addEventListener("click", clearUrl);

function clearUrl() {
  urlInput.value = "";
  urlClear.hidden = true;
  model.clearInfo();
  urlInput.focus();
}

// 창 어디서든 ⌘V 로 붙여넣으면 입력창으로 보낸다.
document.addEventListener("keydown", (e) => {
  if (e.metaKey && e.key.toLowerCase() === "v" && document.activeElement !== urlInput && !model.state.download) {
    urlInput.focus();
  }
  if (e.key === "Escape" && model.state.download) {
    const s = model.state.download.status;
    if (s === "finished" || s === "failed") model.dismissDownload();
  }
});

// ───────── 그리기 ─────────

model.subscribe((s, prev) => {
  renderToolbar(s);
  const ready = s.phase === "ready";
  searchSection.hidden = !ready;
  urlInput.disabled = s.loadingInfo || !!s.download;
  document.body.classList.toggle("loading", s.loadingInfo);

  urlError.hidden = !s.infoError;
  urlError.textContent = s.infoError ?? "";

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
  if (s.download !== prev.download || s === prev) renderSheet(s.download);
  if (prev.phase !== "ready" && ready) urlInput.focus();
});

function renderToolbar(s: State) {
  const items: HTMLElement[] = [];
  if (s.updating) {
    const pct = s.install?.fraction != null ? ` ${Math.round(s.install.fraction * 100)}%` : "";
    items.push(h("span", { class: "toolbar-status" }, h("span", { class: "spinner small" }), `yt-dlp 업데이트 중${pct}`));
  } else if (s.updateError) {
    items.push(
      h(
        "button",
        { class: "toolbar-button warn", title: s.updateError, onclick: () => model.updateNow() },
        icon(icons.warning),
        "업데이트 실패 · 다시 시도",
      ),
    );
  } else if (s.tools?.updateAvailable) {
    items.push(
      h(
        "button",
        {
          class: "toolbar-button accent",
          title: `yt-dlp ${s.tools.latestYtdlpVersion}(으)로 업데이트`,
          onclick: () => model.updateNow(),
          disabled: !!s.download,
        },
        icon(icons.update),
        "업데이트",
      ),
    );
  }
  if (s.tools?.ytdlpVersion) {
    items.push(h("span", { class: "badge", title: "yt-dlp 버전" }, s.tools.ytdlpVersion));
  }
  mount(toolbarItems, ...items);
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

function setupView(s: State) {
  const p = s.install;
  const fraction = p?.stage === "downloading" ? p.fraction : null;
  return h(
    "div",
    { class: "placeholder setup" },
    h("div", { class: "setup-badge" }, icon(icons.play, "icon large")),
    h("h2", null, "처음 실행 준비 중"),
    h("p", { class: "secondary" }, installText(p)),
    h(
      "div",
      { class: `progress ${fraction == null ? "indeterminate" : ""}` },
      h("div", { class: "progress-fill", style: { width: `${(fraction ?? 0) * 100}%` } }),
    ),
    h("p", { class: "footnote" }, "yt-dlp와 ffmpeg를 내려받습니다. 처음 한 번만 필요합니다."),
  );
}

function setupFailedView(s: State) {
  return h(
    "div",
    { class: "placeholder" },
    icon(icons.warning, "icon large warn"),
    h("h2", null, "필요한 구성 요소를 설치하지 못했습니다"),
    h("p", { class: "secondary selectable" }, s.setupError ?? ""),
    h("p", { class: "footnote" }, "인터넷 연결을 확인한 뒤 다시 시도해 주세요."),
    h("button", { class: "push-button default", onclick: () => model.retrySetup() }, "다시 시도"),
  );
}

function loadingView() {
  return h(
    "div",
    { class: "placeholder" },
    h("span", { class: "spinner" }),
    h("p", { class: "secondary" }, "영상 정보를 가져오는 중…"),
  );
}

function emptyView(s: State) {
  return h(
    "div",
    { class: "placeholder" },
    icon(icons.play, "icon large tertiary"),
    h("h2", null, "다운로드할 영상의 링크를 붙여넣으세요"),
    h("p", { class: "secondary" }, h("kbd", null, "⌘"), h("kbd", null, "V"), " 로 붙여넣으면 바로 정보를 가져옵니다."),
    !s.tools?.ffmpegInstalled ? h("p", { class: "footnote warn-text" }, "ffmpeg가 없어 영상+오디오 합치기를 할 수 없습니다.") : null,
  );
}

function infoView(s: State) {
  const info = s.info!;
  const videos = info.formats.filter((f) => f.kind !== "audioOnly");
  const audios = info.formats.filter((f) => f.kind === "audioOnly");
  const meta = [info.uploader, formatDuration(info.duration)].filter(Boolean).join(" · ");

  return h(
    "div",
    { class: "scroll" },
    h(
      "div",
      { class: "video" },
      h(
        "div",
        { class: "thumb" },
        info.thumbnail
          ? h("img", { src: info.thumbnail, alt: "", loading: "lazy", draggable: "false" })
          : icon(icons.play, "icon large tertiary"),
        info.duration ? h("span", { class: "duration" }, formatDuration(info.duration)) : null,
      ),
      h("div", { class: "video-text" }, h("h1", { class: "selectable" }, info.title), meta ? h("p", { class: "secondary" }, meta) : null),
    ),
    info.formats.length === 0 ? h("p", { class: "secondary center" }, "다운로드할 수 있는 형식이 없습니다.") : null,
    videos.length ? section("동영상", icons.film, videos) : null,
    audios.length ? section("오디오", icons.music, audios, "오디오는 변환 없이 원본 형식 그대로 저장합니다.") : null,
  );
}

function section(title: string, svg: string, formats: FormatOption[], footer?: string) {
  return h(
    "section",
    { class: "group" },
    h("h3", { class: "group-title" }, icon(svg, "icon tiny"), title),
    h("div", { class: "group-box", role: "list" }, formats.map(row)),
    footer ? h("p", { class: "group-footer" }, footer) : null,
  );
}

function row(f: FormatOption) {
  return h(
    "button",
    { class: "row", role: "listitem", onclick: () => model.download(f), title: `${f.label} 저장…` },
    h("span", { class: "row-main" }, h("span", { class: "row-title" }, f.label), h("span", { class: "row-detail" }, formatDetail(f))),
    h("span", { class: "row-size" }, formatSize(f)),
    icon(icons.download, "icon download"),
  );
}

// ───────── 다운로드 시트 ─────────

function renderSheet(d: DownloadState | null) {
  if (!d) {
    if (!sheet.hidden) {
      sheet.classList.add("closing");
      setTimeout(() => {
        sheet.hidden = true;
        sheet.classList.remove("closing");
        mount(sheet);
      }, 180);
    }
    return;
  }
  if (sheet.hidden) {
    // 처음 나타날 때만 내려오는 애니메이션을 재생한다 (진행 이벤트마다 다시 그리므로).
    sheet.classList.add("entering");
    setTimeout(() => sheet.classList.remove("entering"), 300);
  }
  sheet.hidden = false;
  sheet.classList.remove("closing");

  const done = d.status === "finished";
  const failed = d.status === "failed";
  const running = d.status === "running" || d.status === "merging";

  let headline = "다운로드 중";
  let detail = "";
  if (d.status === "running") {
    const part = d.stages > 1 ? (d.stage === 0 ? "영상 " : "오디오 ") : "";
    headline = `${part}다운로드 중… ${Math.round(d.overall * 100)}%`;
    detail = [formatSpeed(d.speed), formatEta(d.eta)].filter(Boolean).join(" · ");
  } else if (d.status === "merging") {
    headline = "영상과 오디오를 합치는 중…";
  } else if (done) {
    headline = "다운로드 완료";
    detail = d.path?.split(/[\\/]/).pop() ?? "";
  } else if (failed) {
    headline = "다운로드하지 못했습니다";
    detail = d.error ?? "";
  }

  mount(
    sheet,
    h(
      "div",
      { class: "sheet", role: "dialog", "aria-modal": "true" },
      h(
        "div",
        { class: "sheet-body" },
        done ? icon(icons.check, "icon sheet-icon success") : failed ? icon(icons.warning, "icon sheet-icon warn") : null,
        h("div", { class: "sheet-text" }, h("h2", null, headline), h("p", { class: "sheet-title" }, `${d.title} · ${d.format.label}`)),
      ),
      running
        ? h(
            "div",
            { class: `progress ${d.status === "merging" ? "indeterminate" : ""}` },
            h("div", { class: "progress-fill", style: { width: `${d.overall * 100}%` } }),
          )
        : null,
      detail ? h("p", { class: `sheet-detail ${failed ? "selectable" : ""}` }, detail) : null,
      h(
        "div",
        { class: "sheet-buttons" },
        running ? h("button", { class: "push-button", onclick: () => model.cancelDownload() }, "취소") : null,
        done ? h("button", { class: "push-button", onclick: () => model.revealDownloaded() }, "Finder에서 보기") : null,
        done || failed ? h("button", { class: "push-button default", onclick: () => model.dismissDownload(), autofocus: true }, "완료") : null,
      ),
    ),
  );
}

void model.start();
