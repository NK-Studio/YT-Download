// Windows 11 의 Segoe Fluent Icons 글꼴 글리프를 쓴다.
// 글꼴이 없는 환경(브라우저 미리보기 등)에서는 같은 모양의 SVG 로 대신한다.

const hasFluentFont = /Windows/.test(navigator.userAgent);

const svg = (body: string) =>
  `<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

const table = {
  link: ["", svg('<path d="M8.5 11.5a3 3 0 0 0 4.2 0l2.6-2.6a3 3 0 0 0-4.2-4.2l-.8.8"/><path d="M11.5 8.5a3 3 0 0 0-4.2 0l-2.6 2.6a3 3 0 0 0 4.2 4.2l.8-.8"/>')],
  download: ["", svg('<path d="M10 3v10m0 0l-4-4m4 4l4-4M4 16.5h12"/>')],
  sync: ["", svg('<path d="M16 10a6 6 0 0 1-10.4 4M4 10a6 6 0 0 1 10.4-4"/><path d="M14.5 2.8V6h-3.2M5.5 17.2V14h3.2"/>')],
  video: ["", svg('<rect x="2.5" y="5" width="11" height="10" rx="1.5"/><path d="M13.5 8.5l4-2.5v8l-4-2.5"/>')],
  audio: ["", svg('<path d="M7.5 15V5l9-1.5v10"/><circle cx="5.5" cy="15" r="2"/><circle cx="14.5" cy="13.5" r="2"/>')],
  folder: ["", svg('<path d="M2.5 5.5a1 1 0 0 1 1-1h4l1.5 1.5h7.5a1 1 0 0 1 1 1v8.5a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1z"/>')],
  cancel: ["", svg('<path d="M5 5l10 10M15 5L5 15"/>')],
  check: ["", svg('<path d="M4 10.5l4 4 8-9"/>')],
  info: ["", svg('<circle cx="10" cy="10" r="7.5"/><path d="M10 9v5"/><circle cx="10" cy="6.3" r=".6" fill="currentColor"/>')],
  error: ["", svg('<circle cx="10" cy="10" r="7.5"/><path d="M7.5 7.5l5 5m0-5l-5 5"/>')],
  warning: ["", svg('<path d="M10 3l7.5 13.5h-15z"/><path d="M10 8v4"/><circle cx="10" cy="14.3" r=".6" fill="currentColor"/>')],
  play: ["", svg('<path d="M6 4l10 6-10 6z"/>')],
} as const;

export type IconName = keyof typeof table;

export function fluentIcon(name: IconName, cls = ""): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = `fi ${cls}`;
  el.setAttribute("aria-hidden", "true");
  const [glyph, fallback] = table[name];
  if (hasFluentFont) el.textContent = glyph;
  else el.innerHTML = fallback;
  return el;
}
