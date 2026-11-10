import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

// macOS / Windows 프런트엔드는 서로 다른 폴더(ui/macos, ui/windows)에 두고
// `--mode macos|windows`로 골라 빌드한다. 공통 로직은 @shared(ui/shared)에 있다.
export default defineConfig(({ mode }) => {
  const platform = mode === "windows" ? "windows" : "macos";
  const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));
  return {
    root: r(`./ui/${platform}`),
    base: "./",
    clearScreen: false,
    envDir: r("./"),
    resolve: { alias: { "@shared": r("./ui/shared") } },
    server: {
      port: platform === "windows" ? 1421 : 1420,
      strictPort: true,
      watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
    },
    build: {
      outDir: r(`./dist/${platform}`),
      emptyOutDir: true,
      // macOS는 WKWebView(Safari), Windows는 WebView2(Chromium)
      target: platform === "windows" ? "chrome120" : "safari16",
    },
  };
});
