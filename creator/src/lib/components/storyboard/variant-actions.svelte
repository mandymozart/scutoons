<script lang="ts">
	// The current card's actions (#125; VARIANTS-CANVAS §3.3 – no right-click menu, owner 2026-10-20: "the right click is
	// when you select … the design was better"): under the card, drawn above the row below – square groups. A RESULT (and
	// a dropped file): the STATUS ✓ · ? · ✕ (pressed = its status; pressed again = none – the chosen stays, one per kind;
	// Enter / R / X do the same) and Improve on this · Vary · More ▾ (they fill the Generate panel, nothing is sent). The
	// chosen can't be put in review or rejected (approve another first), a rejected card can't be chosen, put in review,
	// improved or varied (X first), a dropped file (a source) can't be rejected. Under a STRIP too: ✕ pressed, X again
	// brings it back. The render and a take take no status: Improve on (the shot's own inputs) · More.
	import { Icon, Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { GAP_Y, TOP, shortName, statusOf, type BandLayout, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
	import { refusal } from '$lib/editor/storyboard-improve.ts';
	import { refusalText } from '$lib/editor/storyboard-improve.svelte.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import VariantMore from '$lib/components/storyboard/variant-more.svelte';

	let { sb, shot, node, at, layout, meta }: { sb: StoryboardPage; shot: string; node: BoardNode; at: BoardNode; layout: BandLayout; meta: number } = $props();
	const board = $derived(sb.board);
	const r = $derived(node.record);
	const status = $derived(statusOf(r));
	const name = $derived(r ? shortName(r) : node.source === 'take' ? m['storyboard.board_take']({ take: node.take ?? '' }) : m['storyboard.board_render']());
	const kept = $derived(status === 'chosen' ? m['storyboard.board_chosen_kept']({ name }) : '');
	const source = $derived(node.kind === 'source' && status !== 'rejected' ? m['storyboard.board_drop_kept']({ name }) : '');
	const rejectedFirst = $derived(m['storyboard.board_rejected_first']({ name }));
	const options = $derived(
		!r
			? []
			: [
					{ value: 'chosen' as const, icon: 'check', disabled: status === 'rejected' || board.busy, testid: 'status-chosen', title: status === 'chosen' ? m['storyboard.status_chosen_hint']({ name }) : status === 'rejected' ? rejectedFirst : m['storyboard.status_choose']({ name }) },
					{ value: 'review' as const, icon: 'question', disabled: status === 'chosen' || status === 'rejected' || board.busy, testid: 'status-review', title: kept || (status === 'rejected' ? rejectedFirst : m['storyboard.status_review_hint']()) },
					{ value: 'rejected' as const, icon: 'x', disabled: status === 'chosen' || !!source || board.busy, testid: 'status-rejected', title: kept || source || m['storyboard.status_reject_hint']() }
				]
	);
	const noImprove = $derived(refusal(node, 'improve'));
	const noVary = $derived(r ? refusal(node, 'vary', sb.gen.generators) : 'source');
	const improveTitle = $derived(noImprove ? refusalText(noImprove, name) : m['storyboard.improve_hint']({ name }));
	const varyTitle = $derived(noVary ? refusalText(noVary, name) : m['storyboard.vary_hint']({ name }));
	const top = $derived(at.y + TOP + at.h + (at.kind === 'strip' ? 0 : meta) + Math.min(0.25, GAP_Y / 4));
	const set = (s: 'chosen' | 'review' | 'rejected') => void board.setStatus(layout, node, s);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="actions" style:left="{at.x}rem" style:top="{top}rem" data-testid="card-actions" onclick={(e) => e.stopPropagation()}>
	{#if r}
		<Segmented label={m['storyboard.status_label']({ name })} square {options} value={status} onchange={set} onclear={() => status && set(status)} />
	{/if}
	<div class="sc-seg" role="group" aria-label={m['storyboard.card_actions']({ name })}>
		<button type="button" class="is-square" aria-label={improveTitle} title={improveTitle} disabled={!!noImprove} onclick={() => void board.improve.improveOn(shot, node)} data-testid="card-improve"><Icon name="zap" /></button>
		{#if r}
			<button type="button" class="is-square" aria-label={varyTitle} title={varyTitle} disabled={!!noVary} onclick={() => void board.improve.vary(shot, node)} data-testid="card-vary"><Icon name="rotate-cw" /></button>
		{/if}
		<VariantMore {sb} {shot} {node} {name} />
	</div>
</div>

<style>
	.actions {
		position: absolute;
		z-index: 2;
		display: flex;
		gap: var(--space-025);
	}
	.actions:has(:global([aria-expanded='true'])) {
		z-index: 5;
	}
</style>
