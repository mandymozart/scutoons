<script>
	// The current card's actions (agents/research/VARIANTS-CANVAS.md §3.3 – no right-click menu, owner 2026-10-20: "the
	// right click is when you select … the design was better"): under the card, drawn above the row below – square
	// groups. A RESULT (and a dropped file): the STATUS ✓ · ? · ✕ (pressed = the card's status; pressed again = none – the
	// chosen stays, one per kind; Enter / R / X do the same) and Improve on this · Vary · More ▾ (stage 3,
	// improve.svelte.js – they fill the Generate panel, nothing is sent: ⚡ / I the card where its kind fits, ↻ its own
	// inputs with a new seed – a sibling; More ▾ = CardMore). The chosen can't be put in review or rejected (approve
	// another first), a rejected card can't be chosen, put in review, improved or varied (X first – §3.5 "a result
	// wanted back: X on it"), a dropped file (a source) can't be rejected. Under a STRIP too (the current card folded –
	// its X just folded it): ✕ pressed, X again brings it back. A SOURCE takes no status (Q5 A, research/studio-layout/
	// variants-canvas-q5.html): a layout variant's group is Load (into the Viewport) · Take this (it becomes the shot –
	// asks first) · Improve on (its frame as the still, the shot stays) · More; the render's and a take's are Improve on
	// (the shot's own inputs; a take: its render as the video) · More. `node` = the current card (the tree's), `at` = the
	// box that shows it (its card or its strip), `take` = a take's render, `meta` = the caption's height (0 below 60 %).
	import { Icon, Segmented } from '@scutoons/ui';
	import { GAP_Y, TOP, shortId, statusOf } from '../board-layout.js';
	import { board, sourceName } from '../board.svelte.js';
	import { improve, refusal, vary } from '../improve.svelte.js';
	import { fail } from '../studio.svelte.js';
	import CardMore from './CardMore.svelte';

	/** @type {{ shot: string, node: any, at: any, take: any, meta: number, onstatus: (status: string) => void, onaction: (action: string) => void }} */
	let { shot, node, at, take, meta, onstatus, onaction } = $props();
	const r = $derived(node.record);
	const status = $derived(statusOf(r));
	const name = $derived(r ? shortId(r.id) : sourceName(node));
	const kept = $derived(status === 'chosen' ? `${name} is the chosen ${r.kind} – approve another first` : '');
	// a dropped file is a source: not rejectable (§3.5) – one rejected before can come back
	const source = $derived(node.kind === 'source' && status !== 'rejected' ? `${name} is a source (dropped) – it can't be rejected` : '');
	const options = $derived(!r ? [] : [
		{ value: 'chosen', icon: 'check', disabled: !!r.missing || status === 'rejected', testid: 'status-chosen',
			title: status === 'chosen' ? `Chosen – ${r.shot}'s ${r.kind} (one per kind: approving another replaces it)` : r.missing ? `${name}: its file is gone` : status === 'rejected' ? `${name} is rejected – X brings it back first` : `Chosen – approve …: ${name} becomes ${r.shot}'s ${r.kind} (Enter)` },
		{ value: 'review', icon: 'question', disabled: status === 'chosen' || status === 'rejected', testid: 'status-review',
			title: kept || (status === 'rejected' ? `${name} is rejected – X brings it back first` : status === 'review' ? `Review – ${name} is in review; pressed again: none (R)` : 'Review – to look at again / a change asked (R)') },
		{ value: 'rejected', icon: 'x', disabled: status === 'chosen' || !!source, testid: 'status-rejected',
			title: kept || source || (status === 'rejected' ? `Rejected – pressed again brings back what that X rejected (X)` : 'Rejected – with what hangs under it (not the chosen); X again brings them back (X)') }
	]);
	const v = $derived(node.source === 'variant' ? node.variant : null);
	const noImprove = $derived(refusal(node, 'improve'));
	const noVary = $derived(r ? refusal(node, 'vary') : '');
	const HOW = { render: 'the shot\'s own inputs', variant: 'its frame as the still – the shot stays as it is', take: 'its render as the video (video to video)' };
	const improveTitle = $derived(noImprove || `Improve on ${name} – fills Generate: ${HOW[node.source] ?? (r?.kind === 'video' ? 'the video to restyle (+ the chosen image at 0)' : 'the still – the first frame, or the picture ComfyUI edits')}; nothing is sent before Generate (I)`);
	// under the picture and its caption (a strip has none), a hair below (the gap to the row below holds it – GAP_Y)
	const top = $derived(at.y + TOP + at.h + (at.kind === 'strip' ? 0 : meta) + Math.min(0.25, GAP_Y / 4));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="actions" style:left="{at.x}rem" style:top="{top}rem" data-testid="card-actions" onclick={(e) => e.stopPropagation()}>
	{#if v}
		<!-- a layout variant: Load · Take this (today's calls – the retired Variants panel's) -->
		<div class="sc-seg" role="group" aria-label="{v.id}: the layout">
			<button type="button" class="is-square" aria-label="Load {v.id}" title="Load – {v.id} into the Viewport to change it (unsaved: the shot stays until Save + frame)" disabled={board.busy} onclick={() => onaction('load')} data-testid="variant-load"><Icon name="box" /></button>
			<button type="button" class="is-square" aria-label="Take {v.id}" title="Take this – {v.id} becomes the shot, the shot as saved now a variant (asks first)" disabled={board.busy} onclick={() => onaction('take')} data-testid="variant-take"><Icon name="swap" /></button>
		</div>
	{:else if r}
		<Segmented label="{name}: status – the pressed one; pressed again: none" square {options} value={status} onchange={onstatus} onclear={() => onstatus(status)} />
	{/if}
	<div class="sc-seg" role="group" aria-label="{name}'s actions">
		<button type="button" class="is-square" aria-label={improveTitle} title={improveTitle} disabled={!!noImprove} onclick={() => improve(shot, node, take).catch(fail)} data-testid="card-improve"><Icon name="zap" /></button>
		{#if r}
			{@const varyTitle = noVary || `Vary ${name} – its own inputs, prompt and model with a new seed: a sibling in its row; fills Generate, nothing is sent before Generate`}
			<button type="button" class="is-square" aria-label={varyTitle} title={varyTitle} disabled={!!noVary} onclick={() => vary(shot, node).catch(fail)} data-testid="card-vary"><Icon name="rotate-cw" /></button>
		{/if}
		<CardMore {shot} {node} {take} {name} />
	</div>
</div>

<style>
	.actions {
		position: absolute;
		z-index: 2;
		display: flex;
		gap: var(--space-025);
	}
	/* More ▾ open: its list over the next band's sticky head too */
	.actions:has(:global([aria-expanded='true'])) {
		z-index: 5;
	}
</style>
