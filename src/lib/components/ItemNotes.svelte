<script lang="ts">
  // The player's own note on an item: kept on this PC and shown whenever the item is checked or
  // looked up again. One note per item name; a slightly misread name finds the same note.
  import NoteBox from "./NoteBox.svelte";
  import { backend } from "$lib/api";

  let { name }: { name: string } = $props();
</script>

<NoteBox
  key={name || null}
  load={() => backend.itemNote(name)}
  save={(text) => backend.setItemNote(name, text)}
  placeholder="What to remember about it: keep, sell, scroll it, who wants one"
/>
