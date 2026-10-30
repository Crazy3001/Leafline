<script lang="ts">
  let { size = 18 }: { size?: number } = $props();
</script>

<!-- A simple leaf mark drawn for this project. -->
<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  <path d="M4.5 19.5C4 11 9.5 4.5 20 4c.4 9.8-5.6 15.6-15.5 15.5Z" fill="var(--accent)" />
  <path d="M4.5 19.5 14 10" stroke="#2a1a05" stroke-opacity="0.6" stroke-width="1.6" stroke-linecap="square" />
</svg>
