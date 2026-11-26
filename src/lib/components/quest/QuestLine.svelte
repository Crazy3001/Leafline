<script lang="ts">
  // The questline a quest is part of, in the Quests page's detail pane: every step, with what's
  // done, the step you're on and the one after it. Each step opens in its place.
  import { Check, CircleCheck, CircleDot, Lock, Undo2 } from "@lucide/svelte";

  import Card from "../Card.svelte";
  import { currentStep, isReady, lineSteps } from "./questbook";
  import type { QuestReading, Stand, WholeLog } from "$lib/types";

  let {
    whole,
    id,
    helper = null,
    onpick,
  }: {
    whole: WholeLog | null;
    /** The quest open in the pane. */
    id: number;
    helper?: QuestReading | null;
    onpick: (id: number) => void;
  } = $props();

  const all = $derived((whole?.regions ?? []).flatMap((r) => r.quests));
  const head = $derived(all.find((q) => q.questId === id)?.line?.head ?? null);
  const steps = $derived(head == null ? [] : lineSteps(all.filter((q) => q.line?.head === head && q.questId != null)));
  const current = $derived(currentStep(steps, helper));
  const currentAt = $derived(current ? steps.indexOf(current) : -1);
  const done = $derived(steps.filter((q) => q.stand === "done" || q.stand === "gone").length);
  const name = $derived(steps[0]?.line?.name ?? "");

  const standOf = (stand: Stand, ready: boolean): Stand => (stand === "inProgress" && ready ? "ready" : stand);
  /** What a step's tag says: where it stands against the step you're on. */
  function tag(i: number, stand: Stand): { text: string; tone: string } | null {
    if (stand === "done" || stand === "gone") return { text: "Done", tone: "good" };
    if (i === currentAt) {
      // Ready to hand in is said by its icon here and in the quest's card below.
      if (stand === "ready" || stand === "inProgress") return { text: "On now", tone: "now" };
      return { text: "Up next", tone: "now" };
    }
    if (i === currentAt + 1) return { text: "Next", tone: "next" };
    return null;
  }
</script>

{#if steps.length > 1}
  <Card title="Questline">
    {#snippet aside()}<span class="tabular">{done} of {steps.length} done</span>{/snippet}
    <h3 class="line-name">{name}</h3>
    <ol class="steps">
      {#each steps as q, i (q.questId)}
        {@const stand = standOf(q.stand, isReady(q, helper))}
        {@const t = tag(i, stand)}
        <li class="stand-{stand}" class:current={i === currentAt} class:open={q.questId === id}>
          <span class="icon" aria-hidden="true">
            {#if stand === "done"}<Check size={12} strokeWidth={2.6} />
            {:else if stand === "ready"}<CircleCheck size={12} strokeWidth={2.4} />
            {:else if stand === "inProgress"}<CircleDot size={12} strokeWidth={2.4} />
            {:else if stand === "gone"}<Undo2 size={12} strokeWidth={2.4} />
            {:else if stand === "notYet"}<Lock size={11} strokeWidth={2.4} />
            {:else}<span class="num tabular">{i + 1}</span>{/if}
          </span>
          <button class="pick" aria-current={q.questId === id || undefined} onclick={() => onpick(q.questId!)}>
            <span class="name">{q.title}</span>
            {#if q.levelMin != null && stand !== "done"}<span class="lv">Lv {q.levelMin}</span>{/if}
          </button>
          {#if t}<span class="tag {t.tone}">{t.text}</span>{/if}
        </li>
      {/each}
    </ol>
  </Card>
{/if}

<style>
  .line-name {
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .steps {
    position: relative;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .steps li {
    position: relative;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
  }

  /* The thread between steps. */
  .steps li + li::before {
    content: "";
    position: absolute;
    top: -6px;
    left: calc(var(--space-2) + 9px);
    width: 2px;
    height: 10px;
    border-radius: 1px;
    background: var(--border-strong);
  }

  .steps li.current {
    background: color-mix(in srgb, var(--accent) 13%, transparent);
  }

  .steps li.open:not(.current) {
    background: var(--surface-2);
  }

  .icon {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--faint);
  }

  .num {
    font-size: var(--text-xs);
  }

  .stand-done .icon,
  .stand-ready .icon {
    background: color-mix(in srgb, var(--good) 20%, transparent);
    color: var(--good);
  }

  .stand-inProgress .icon {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent-text);
  }

  .pick {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 var(--space-2);
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .pick:hover .name,
  .pick:focus-visible .name {
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    outline: none;
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .stand-done .name,
  .stand-gone .name {
    color: var(--muted);
    font-weight: var(--weight-body);
  }

  .lv {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .tag {
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .tag.good {
    color: var(--good);
  }

  .tag.now {
    color: var(--accent-text);
    font-weight: var(--weight-strong);
  }

  .tag.next {
    color: var(--muted);
  }
</style>
