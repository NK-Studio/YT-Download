// SF Symbols 느낌의 단순한 아이콘 (WebView에서는 SF Symbols를 쓸 수 없다).
const svg = (body: string, vb = "0 0 24 24") =>
  `<svg viewBox="${vb}" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const icons = {
  link: svg('<path d="M10 14a4.5 4.5 0 0 0 6.4 0l3.2-3.2a4.5 4.5 0 0 0-6.4-6.4l-1 1"/><path d="M14 10a4.5 4.5 0 0 0-6.4 0l-3.2 3.2a4.5 4.5 0 0 0 6.4 6.4l1-1"/>'),
  clear: `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" fill="currentColor"/><path d="M8.5 8.5l7 7m0-7l-7 7" stroke="var(--window)" stroke-width="2" stroke-linecap="round"/></svg>`,
  download: `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10.5" fill="currentColor"/><path d="M12 6.8v9m0 0l-3.8-3.8M12 15.8l3.8-3.8" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>`,
  update: svg('<circle cx="12" cy="12" r="9"/><path d="M12 7.5v8m0 0l-3.2-3.2M12 15.5l3.2-3.2"/>'),
  film: svg('<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M7.5 4.5v15M16.5 4.5v15M3 9.5h4.5M3 14.5h4.5M16.5 9.5H21M16.5 14.5H21"/>'),
  music: svg('<path d="M9 18V6l11-2v12"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="17.5" cy="16" r="2.5"/>'),
  play: svg('<rect x="2.5" y="5" width="19" height="14" rx="4"/><path d="M10 9.2v5.6l4.8-2.8z" fill="currentColor"/>'),
  check: `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="11" fill="currentColor"/><path d="M7 12.5l3.2 3.2L17 9" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>`,
  warning: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10.3 3.6a2 2 0 0 1 3.4 0l8.4 14.6A2 2 0 0 1 20.4 21H3.6a2 2 0 0 1-1.7-2.8z" fill="currentColor"/><path d="M12 9v5" stroke="#fff" stroke-width="2.2" stroke-linecap="round"/><circle cx="12" cy="17.3" r="1.3" fill="#fff"/></svg>`,
  chevron: svg('<path d="M9 6l6 6-6 6"/>'),
};
