<script>
	// The current card's actions (agents/research/VARIANTS-CANVAS.md §3.3 – no right-click menu, owner 2026-10-20: "the
	// right click is when you select … the design was better"): under the card, drawn above the row below – two square
	// groups: the STATUS ✓ · ? · ✕ (pressed = the card's status; pressed again = none – the chosen stays, one per kind;
	// Enter / R / X do the same) and Improve on this · Vary · More (stage 3 – here as disabled placeholders). The render
	// and the takes have no status; the chosen can't be put in review or rejected (approve another first), a rejected card
	// can't be chosen (X first), a dropped file (a source) can't be rejected.
	import { Segmented } from '@scutoons/ui';
	import { GAP_Y, META, TOP, shortId, statusOf } from '../board-layout.js';

	/** @type {{ node: any, onstatus: (status: string) => void }} */
	let { node, onstatus } = $props();
	const r = $derived(node.record);
	const status = $derived(statusOf(r));
	const name = $derived(r ? shortId(r.id) : node.source === 'take' ? `take ${node.take}` : 'the render');
	const none = $derived(!r ? `${name} has no status – it is what the generators get` : '');
	const kept = $derived(status === 'chosen' ? `${name} is the chosen ${r.kind} – approve another first` : '');
	// a dropped file is a source: not rejectable (§3.5) – one rejected before can come back
	const source = $derived(node.kind === 'source' && status !== 'rejected' ? `${name} is a source (dropped) – it can't be rejected` : '');
	const options = $derived([
		{ value: 'chosen', icon: 'check', disabled: !r || !!r.missing || status === 'rejected', testid: 'status-chosen',
			title: none || (status === 'chosen' ? `Chosen – ${r.shot}'s ${r.kind} (one per kind: approving another replaces it)` : r.missing ? `${name}: its file is gone` : status === 'rejected' ? `${name} is rejected – X brings it back first` : `Chosen – approve …: ${name} becomes ${r.shot}'s ${r.kind} (Enter)`) },
		{ value: 'review', icon: 'question', disabled: !r || status === 'chosen', testid: 'status-review',
			title: none || kept || (status === 'review' ? `Review – ${name} is in review; pressed again: none (R)` : 'Review – to look at again / a change asked (R)') },
		{ value: 'rejected', icon: 'x', disabled: !r || status === 'chosen' || !!source, testid: 'status-rejected',
			title: none || kept || source || (status === 'rejected' ? `Rejected – pressed again brings back what that X rejected (X)` : 'Rejected – with what hangs under it (not the chosen); X again brings them back (X)') }
	]);
	const LATER = [
		{ value: 'improve', icon: 'zap', title: 'Improve on this – stage 3 (not built yet)', disabled: true },
		{ value: 'vary', icon: 'rotate-cw', title: 'Vary – its inputs, prompt and model with a new seed – stage 3 (not built yet)', disabled: true },
		{ value: 'more', icon: 'more-horizontal', title: 'More – Compare, a frame as the still, show the file, delete – stage 3 (not built yet)', disabled: true }
	];
	// under the picture and its caption, a hair below (the gap to the row below holds it – board-layout GAP_Y)
	const top = $derived(node.y + TOP + node.h + META + Math.min(0.25, GAP_Y / 4));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="actions" style:left="{node.x}rem" style:top="{top}rem" data-testid="card-actions" onclick={(e) => e.stopPropagation()}>
	<Segmented label="{name}: status – the pressed one; pressed again: none" square {options} value={status} onchange={onstatus} onclear={() => onstatus(status)} />
	<Segmented label="{name}'s actions" square options={LATER} value={null} onchange={() => {}} />
</div>

<style>
	.actions {
		position: absolute;
		z-index: 2;
		display: flex;
		gap: var(--space-025);
	}
</style>
