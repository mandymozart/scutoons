<script lang="ts">
	// More ▾ – the current card's third action (#125; VARIANTS-CANVAS §3.3, §3.6): a dropdown from its button (@scutoons/ui
	// Menu), never a right-click menu. Its items:
	//   - an IMAGE: Use as start frame / keyframe – it ADDS the card to the Generate form (the still), the rest stays;
	//     offered only where the form's model takes it as one (Gen-4: the first frame; Aleph 2: the keyframe at 0);
	//   - a VIDEO: Still – the first frame | the frame pinned by a click on the scrubbing card | the last frame: cut here in
	//     the browser, recorded as an image of the shot, then the form's still (§3.7 frame → still);
	//   - Add to / Remove from Compare (= the board's selection, Ctrl + click; a rejected card stays out);
	//   - Show the file (a new tab).
	// Delete stays out (phase 5). Rejected cards: nothing goes into the form (§3.5).
	import { tick } from 'svelte';
	import { Icon, Menu } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { statusOf, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
	import { frameStillOf, refusal, useAsOf } from '$lib/editor/storyboard-improve.ts';
	import { refusalText } from '$lib/editor/storyboard-improve.svelte.ts';
	import { secondsText } from '$lib/editor/storyboard-posters.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb, shot, node, name }: { sb: StoryboardPage; shot: string; node: BoardNode; name: string } = $props();
	const board = $derived(sb.board);
	const r = $derived(node.record);
	const g = $derived(r ? (sb.generations.find((x) => x.id === r.id) ?? null) : null);
	const refused = $derived(refusal(node, 'use'));
	const why = $derived(refused ? refusalText(refused, name) : '');
	const model = $derived(sb.gen.model);
	const as = $derived(useAsOf(model));
	const frame = $derived(frameStillOf(model));
	const pin = $derived(board.pin?.key === node.key ? board.pin.ms : null);
	const picked = $derived(board.selected.some((s) => s?.key === node.key));
	const rejected = $derived(statusOf(r) === 'rejected');
	let file = $state<string | null>(null);
	$effect(() => {
		const gen = g;
		file = null;
		if (gen) void board.fileOf(gen).then((u) => (file = u));
	});
	const noModel = $derived(m['storyboard.more_no_model']({ model: model?.label ?? '–' }));
	let host = $state<HTMLElement>();
	const refocus = () => void tick().then(() => host?.querySelector<HTMLElement>('.trigger')?.focus({ preventScroll: true }));
	const run = (p: Promise<void>) => {
		refocus();
		void p.finally(refocus);
	};
	let drop = $state<'down' | 'up'>('down');
	function measure(e: MouseEvent) {
		const button = (e.currentTarget as HTMLElement).querySelector('.trigger');
		const view = (e.currentTarget as HTMLElement).closest('[data-board-scroll]')?.getBoundingClientRect();
		if (!button || !view || button.getAttribute('aria-expanded') === 'true') return;
		const b = button.getBoundingClientRect();
		const need = 5.5 * b.height;
		drop = b.bottom + need > view.bottom && b.top - need > view.top ? 'up' : 'down';
	}
</script>

<span class="more" bind:this={host} onclickcapture={measure}>
	<Menu label={m['storyboard.more_label']({ name })} {drop}>
		{#snippet trigger()}<Icon name="more-horizontal" />{/snippet}
		{#if r && g?.kind === 'video'}
			<button type="button" disabled={!!refused || !frame || board.improve.cutting} title={why || (frame ? m['storyboard.more_frame_hint']({ name }) : noModel)} onclick={() => run(board.improve.useFrame(shot, node, 'first'))} data-testid="more-first">{m['storyboard.more_first']()}</button>
			<button type="button" disabled={!!refused || !frame || pin == null || board.improve.cutting} title={pin == null ? m['storyboard.more_pin_hint']({ name }) : why || (frame ? m['storyboard.more_frame_hint']({ name }) : noModel)} onclick={() => pin != null && run(board.improve.useFrame(shot, node, pin))} data-testid="more-pinned">{pin == null ? m['storyboard.more_at_unset']() : m['storyboard.more_at']({ time: secondsText(pin) })}</button>
			<button type="button" disabled={!!refused || !frame || board.improve.cutting} title={why || (frame ? m['storyboard.more_frame_hint']({ name }) : noModel)} onclick={() => run(board.improve.useFrame(shot, node, 'last'))} data-testid="more-last">{m['storyboard.more_last']()}</button>
		{:else if r}
			<button type="button" disabled={!!refused || !as} title={why || (as ? m['storyboard.more_use_hint']({ name }) : noModel)} onclick={() => run(board.improve.useAs(shot, node))} data-testid="more-use">{as === 'keyframe' ? m['storyboard.more_use_keyframe']() : m['storyboard.more_use_start']()}</button>
		{/if}
		<button type="button" disabled={rejected && !picked} title={rejected && !picked ? m['storyboard.board_rejected_out']({ name }) : m['storyboard.more_compare_hint']()} onclick={() => { board.toggleSelected({ shot, key: node.key }, node); refocus(); }} data-testid="more-compare">{picked ? m['storyboard.more_compare_out']() : m['storyboard.more_compare_in']()}</button>
		{#if r}
			<button type="button" disabled={!file} title={file ? (g?.file ?? '') : m['storyboard.board_no_file']()} onclick={() => { if (file) window.open(file, '_blank', 'noopener'); refocus(); }} data-testid="more-file">{m['storyboard.more_file']()}</button>
		{/if}
	</Menu>
</span>

<style>
	.more {
		display: inline-flex;
	}
	.more :global(.sc-menu > .trigger) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: calc(var(--control-height) - 2 * var(--border-width-regular));
		padding: 0;
		border: var(--border-width-regular) solid transparent;
		background: transparent;
		color: var(--ink);
	}
	.more :global(.sc-menu > .trigger:hover) {
		border-color: var(--border-muted);
	}
	.more :global(.sc-menu > .trigger[aria-expanded='true']) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.more :global(.popup > button) {
		white-space: nowrap;
	}
</style>
