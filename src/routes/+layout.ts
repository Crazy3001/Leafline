// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;

// The website writes a page for each route, so a link to one answers as itself rather than
// through GitHub Pages' "not found" page. The program loads its routes through the fallback.
export const prerender = __LEAFLINE_WEB__;
