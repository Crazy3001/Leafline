// The website's text size zooms the whole page with CSS `zoom`. A fixed box's `left` and `top`
// are then scaled by it, while the mouse, element boxes and the window are measured unzoomed: a
// box placed from them lands too far right and down. Anything placed by those measures divides by
// this first. The program zooms its webview instead, so there it is 1.

export function pageZoom(): number {
  const root = document.documentElement;
  return parseFloat(root.style.zoom || getComputedStyle(root).zoom) || 1;
}
