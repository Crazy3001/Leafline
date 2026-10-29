<script lang="ts">
  let { size = 18 }: { size?: number } = $props();
</script>

<!-- A simple leaf mark drawn for this project. -->
<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  <defs>
    <linearGradient id="leaf" x1="4" y1="20" x2="20" y2="4" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f08a24" />
      <stop offset="1" stop-color="#ffd166" />
    </linearGradient>
  </defs>
  <path d="M4.5 19.5C4 11 9.5 4.5 20 4c.4 9.8-5.6 15.6-15.5 15.5Z" fill="url(#leaf)" />
  <path d="M4.5 19.5 14 10" stroke="#3b2205" stroke-opacity="0.55" stroke-width="1.6" stroke-linecap="round" />
</svg>
