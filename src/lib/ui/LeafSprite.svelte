<script lang="ts">
  // Leafline's own little sprout, drawn here: it stands in for the player's character, since
  // Leafline has no picture of them. Original art, so nothing of the game's is copied.
  let { size = 64 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 80 80" aria-hidden="true">
  <ellipse cx="40" cy="72" rx="20" ry="4" fill="#000" opacity="0.18" />
  <path d="M22 66 Q18 34 40 30 Q62 34 58 66Z" fill="#9be38c" />
  <path d="M24 64 Q40 70 56 64" stroke="#5aa95a" stroke-width="2" fill="none" />
  <circle cx="33" cy="48" r="3.4" fill="#173a1f" />
  <circle cx="47" cy="48" r="3.4" fill="#173a1f" />
  <circle cx="34" cy="47" r="1.1" fill="#fff" />
  <circle cx="48" cy="47" r="1.1" fill="#fff" />
  <path d="M36 55 Q40 58 44 55" stroke="#173a1f" stroke-width="2" fill="none" stroke-linecap="round" />
  <ellipse cx="28" cy="54" rx="3.5" ry="2" fill="#f59f8b" opacity="0.7" />
  <ellipse cx="52" cy="54" rx="3.5" ry="2" fill="#f59f8b" opacity="0.7" />
  <path d="M40 30 Q26 6 12 14 Q24 24 40 30Z" fill="#4caf50" />
  <path d="M40 30 Q56 4 70 12 Q56 24 40 30Z" fill="#66c56a" />
</svg>
