// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
//
// The website (`npm run build:web`, which sets LEAFLINE_TARGET=web) is the same pages served from
// GitHub Pages at crazy3001.github.io/Leafline: hence its base path and a 404.html fallback, which
// Pages serves for any address it doesn't have.
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import process from "node:process";

const web = process.env.LEAFLINE_TARGET === "web";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(
      web ? { pages: "build-web", assets: "build-web", fallback: "404.html" } : { fallback: "index.html" },
    ),
    paths: { base: web ? "/Leafline" : "" },
    // The website writes a page per route (see routes/+layout.ts). The program's own windows with
    // a parameter in the address (a card, a pinned panel) aren't linked on the website.
    prerender: { handleUnseenRoutes: "ignore" },
  },
};

export default config;
