import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [sveltekit()],

  // The website is Vite's "web" mode. A constant, so the program's build drops the website's code.
  define: { __LEAFLINE_WEB__: JSON.stringify(mode === "web") },
  // Its own dependency cache, so the website's dev server (npm run dev:web) can run beside the
  // program's without rewriting the pre-bundled packages under the program's open windows.
  cacheDir: mode === "web" ? "node_modules/.vite-web" : "node_modules/.vite",

  // WebView2 and WKWebView both support top-level await, used to pick the backend.
  build: { target: "es2022" },
  // The website's worker imports its WebAssembly glue as a module.
  worker: { format: "es" },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching the Rust code and the build folders
      ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**", "**/build-web/**"],
    },
  },
}));
