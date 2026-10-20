<script lang="ts">
	// The variants board's tool row (#125; VARIANTS-CANVAS §3.2, §3.5): the editor switch (V – `lead`), "Compare · n" – a
	// square toggle (C; shown once a card is selected or Compare is open), the keys as one line (the whole list in its
	// tooltip), "Show rejected · n" (hidden at 0, not remembered), the zoom − · % · + (the % back to 100 %; 0 fits) and +
	// (round): images or videos for the current shot – the keys' way to what a drop on a band does.
	import type { Snippet } from 'svelte';
	import { Icon, IconButton } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { ACCEPT } from '$lib/editor/storyboard-generate.ts';
	import { percent, zoomStep, ZOOM_MAX, ZOOM_MIN } from '$lib/editor/storyboard-board-zoom.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb, lead, rejected, selected, oncompare, onzoom, onfiles }: { sb: StoryboardPage; lead: Snippet; rejected: number; selected: number; oncompare: () => void; onzoom: (z: number) => void; onfiles: (files: File[]) => void } = $props();
	const board = $derived(sb.board);
	let picker = $state<HTMLInputElement>();
</script>

<div class="tools">
	{@render lead()}
	{#if selected || board.compare}
		<div class="sc-seg">
			<button type="button" aria-pressed={board.compare} title={board.compare ? m['storyboard.compare_close']() : m['storyboard.compare_open_hint']({ n: selected })} onclick={oncompare} data-testid="compare-toggle"><Icon name="columns" />{m['storyboard.compare_n']({ n: selected })}</button>
		</div>
	{/if}
	<span class="hint" title={m['storyboard.board_keys']()}><span>{m['storyboard.board_keys_short']()}</span></span>
	{#if rejected}
		<div class="sc-seg">
			<button type="button" aria-pressed={board.showRejected} title={m['storyboard.board_show_rejected_hint']()} onclick={() => (board.showRejected = !board.showRejected)} data-testid="show-rejected">{m['storyboard.board_show_rejected']({ n: rejected })}</button>
		</div>
	{/if}
	<div class="sc-seg" role="group" aria-label={m['storyboard.board_zoom']()}>
		<button type="button" class="is-square" aria-label={m['storyboard.board_zoom_out']()} title={m['storyboard.board_zoom_out']()} disabled={board.zoom <= ZOOM_MIN} onclick={() => onzoom(zoomStep(board.zoom, -1))}><Icon name="minus" /></button>
		<button type="button" class="pct" title={m['storyboard.board_zoom_hint']()} onclick={() => onzoom(1)} data-testid="board-zoom">{percent(board.zoom)}</button>
		<button type="button" class="is-square" aria-label={m['storyboard.board_zoom_in']()} title={m['storyboard.board_zoom_in']()} disabled={board.zoom >= ZOOM_MAX} onclick={() => onzoom(zoomStep(board.zoom, 1))}><Icon name="plus" /></button>
	</div>
	{#if sb.current}
		<IconButton label={m['storyboard.board_add']({ shot: sb.current })} class="sc-round" onclick={() => picker?.click()} data-testid="board-add"><Icon name="upload" /></IconButton>
		<input bind:this={picker} type="file" accept={ACCEPT} multiple hidden onchange={(e) => { onfiles([...(e.currentTarget.files ?? [])]); e.currentTarget.value = ''; }} />
	{/if}
</div>

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		container-type: inline-size;
	}
	.hint {
		flex: 1 1 0;
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	@container (max-width: 40rem) {
		.hint > span {
			display: none;
		}
	}
	.sc-seg > button {
		white-space: nowrap;
	}
	.pct {
		min-width: 4.5rem;
		font-variant-numeric: tabular-nums;
	}
</style>
