<script lang="ts">
	import { Chip } from '@scutoons/ui/editor';
	import type { PublishLine } from '$lib/editor/publish-tree.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A line of the Publish dialog (RULES H18): the tick, the name, what its draft does as a chip, its kind; the rest is its
	 * hover text. A key the server refuses shows the refusal and its reason instead, unticked.
	 */
	let { line, on, disabled = false, title, ontick }: { line: PublishLine; on: boolean; disabled?: boolean; title?: string; ontick: (on: boolean) => void } = $props();
	const off = $derived(disabled || !!line.refused);
</script>

<label class="line" title={line.refused ? `${line.refused} · ${title ?? ''}` : title} data-testid="publish-line" data-refused={line.refused ? '' : undefined}>
	<input type="checkbox" checked={on} disabled={off} onchange={(e) => ontick(e.currentTarget.checked)} />
	<span class="name">{line.label}</span>
	{#if line.refused}
		<Chip tone="error" label={m['shell.publish_refused']()} />
		<span class="reason">{line.refused}</span>
	{:else}
		<Chip label={line.chip} tabindex={-1} {disabled} onclick={() => ontick(!on)} />
		<span class="type">{line.type}</span>
	{/if}
</label>

<style>
	.line {
		display: flex;
		align-items: center;
		gap: var(--kit-gap);
		min-height: var(--editor-row);
		cursor: pointer;
	}
	/* the tick: the control's line, ticked the pressed fill (RULES H11 – no hue) */
	.line input {
		appearance: none;
		display: inline-grid;
		flex: none;
		place-items: center;
		width: 1rem;
		height: 1rem; /* control-ok: a tick box, not a single-line control */
		margin: 0;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: calc(var(--editor-radius) / 2);
		background: var(--kit-field-rest-ground);
		cursor: pointer;
	}
	.line input:checked {
		border-color: var(--tool-pressed);
		background: var(--tool-pressed);
	}
	.line input:checked::after {
		content: '';
		width: 0.3rem;
		height: 0.55rem; /* control-ok: the tick's mark */
		margin-top: -0.12rem;
		border: solid var(--tool-pressed-text);
		border-width: 0 calc(2 * var(--editor-edge)) calc(2 * var(--editor-edge)) 0;
		transform: rotate(45deg);
	}
	.line input:focus-visible {
		outline: var(--editor-edge) solid var(--tool-text);
		outline-offset: var(--editor-edge);
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.type {
		flex: none;
		margin-left: auto;
		color: var(--tool-muted);
	}
	.reason {
		min-width: 0;
		margin-left: auto;
		overflow: hidden;
		color: var(--tool-danger);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
