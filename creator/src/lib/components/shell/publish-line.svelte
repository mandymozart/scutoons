<script lang="ts">
	import { Chip, Toggle } from '@scutoons/ui/editor';
	import type { PublishLine } from '$lib/editor/publish-tree.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A line of the Publish dialog (RULES H18): the kit's Toggle, the name, what its draft does as a chip, its kind; the
	 * rest is its hover text. A key the server refuses shows the refusal and its reason instead, off. The line is the
	 * Toggle's label: a click anywhere on it flips the switch, the switch alone takes the keys.
	 */
	let { line, on, disabled = false, title, ontick }: { line: PublishLine; on: boolean; disabled?: boolean; title?: string; ontick: (on: boolean) => void } = $props();
	const off = $derived(disabled || !!line.refused);
	const hover = $derived(line.refused ? `${line.refused} · ${title ?? ''}` : title);
</script>

<label class="line" title={hover} data-testid="publish-line" data-refused={line.refused ? '' : undefined}>
	<Toggle {on} label={line.label} title={hover ?? line.label} disabled={off} onchange={ontick} data-testid="publish-tick" />
	<span class="name">{line.label}</span>
	{#if line.refused}
		<Chip tone="error" label={m['shell.publish_refused']()} />
		<span class="reason">{line.refused}</span>
	{:else}
		<Chip label={line.chip} />
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
