<script>
	// More ▾ – the current card's third action (agents/research/VARIANTS-CANVAS.md §3.3, §3.6; issue #62 stage 3): a
	// dropdown from its button (@scutoons/ui Menu), never a right-click menu (owner 2026-10-20: "the right click is when
	// you select … the design was better"); it sits in the card's square group (CardActions). Its items:
	//   - an IMAGE (a result, a dropped image, the render, a layout variant's frame): Use as start frame / keyframe – it
	//     ADDS the card to the Generate form (the still), the rest stays; offered only where the form's model takes it
	//     (gen4: the first frame; Aleph 2: one keyframe at 0 today – GENERATORS §6 step 5 brings ≤ 5), else disabled with why;
	//   - a VIDEO (a result, a dropped clip, a take's render): Still – the first frame | the frame pinned by a click on the
	//     scrubbing card | the last frame: the frame becomes the form's still (§3.7 frame → still, cut by the server);
	//   - Add to / Remove from Compare (= the board's selection, Ctrl + click; a rejected card stays out);
	//   - Show the file (its URL in a new tab).
	// Delete stays out (phase 5). Rejected cards: nothing goes into the form (§3.5 – refused).
	import { tick } from 'svelte';
	import { Icon, Menu } from '@scutoons/ui';
	import { statusOf } from '../board-layout.js';
	import { board, toggleSelected } from '../board.svelte.js';
	import { current, model } from '../generate.svelte.js';
	import { cardOf, frameStillOf, useAsOf } from '../improve.js';
	import { refusal, useAs, useFrame } from '../improve.svelte.js';
	import { secondsText } from '../posters.js';
	import { ch, fail } from '../studio.svelte.js';

	/** @type {{ shot: string, node: any, take: any, name: string }} */
	let { shot, node, take, name } = $props();
	const card = $derived(cardOf(node, { ch: ch(), shot, take }));
	const refused = $derived(refusal(node, 'use'));
	const m = $derived(model());
	const what = $derived(`${current()?.label ?? ''} – ${m?.label ?? ''}`);
	const none = { ok: false, why: 'Generate has no model yet – open it once' };
	const as = $derived(m ? useAsOf(m, what) : { ...none, label: 'Use as start frame / keyframe' });
	const frame = $derived(m ? frameStillOf(m, what) : none);
	const pin = $derived(board.pin?.key === node.key ? board.pin.ms : null);
	const picked = $derived(board.selected.some((s) => s.key === node.key));
	const rejected = $derived(statusOf(node.record) === 'rejected');
	const url = $derived(node.record?.missing ? null : card?.url ?? null);
	/** a frame item's tooltip: why not, or what it does */
	const frameTitle = (words) => refused || frame.why || `${words} of ${name} as Generate's still (${what}) – nothing is sent before Generate`;
	/** an item acted: the focus back on the More button – in the board, so its keys go on working (the list is gone, the
	 *  focus would fall to the page) */
	let host = $state();
	const refocus = () => tick().then(() => host?.querySelector('.trigger')?.focus({ preventScroll: true }));
	const run = (p) => { refocus(); return p.catch(fail).finally(refocus); };
	/** the list opens upwards when the board's view has no room for it below the button (it would hide under the
	 *  timeline) – measured as it opens */
	let drop = $state('down');
	function measure(e) {
		const button = e.currentTarget.querySelector('.trigger');
		const view = e.currentTarget.closest('[data-board-scroll]')?.getBoundingClientRect();
		if (!button || !view || button.getAttribute('aria-expanded') === 'true') return;
		const r = button.getBoundingClientRect();
		const need = 5.5 * r.height;
		drop = r.bottom + need > view.bottom && r.top - need > view.top ? 'up' : 'down';
	}
</script>

<!-- no tooltip on the trigger: shown on its focus, it would lie over the list's first item (the house Menus have none) -->
<span class="more" bind:this={host} onclickcapture={measure}>
	<Menu label="More – {name}: {card?.video ? 'a frame as the still' : 'use as start frame / keyframe'}, Compare, show the file" {drop}>
		{#snippet trigger()}<Icon name="more-horizontal" />{/snippet}
		{#if card?.video}
			<button type="button" disabled={!!refused || !frame.ok} title={frameTitle('The first frame')} onclick={() => run(useFrame(shot, node, take, 'first'))} data-testid="more-first">Still: the first frame</button>
			<button type="button" disabled={!!refused || !frame.ok || pin == null} title={pin == null ? `Click ${name} where you want the frame (it scrubs under the pointer) – that moment is pinned` : frameTitle(`The frame at ${secondsText(pin)}`)} onclick={() => run(useFrame(shot, node, take, pin))} data-testid="more-pinned">Still: the frame at {pin == null ? '… (click the card)' : secondsText(pin)}</button>
			<button type="button" disabled={!!refused || !frame.ok} title={frameTitle('The last frame')} onclick={() => run(useFrame(shot, node, take, 'last'))} data-testid="more-last">Still: the last frame</button>
		{:else if card}
			<button type="button" disabled={!!refused || !as.ok} title={refused || as.why || `${name} as Generate's ${m?.startFrame ? 'start frame' : 'keyframe at 0'} (${what}) – the rest of the form stays; nothing is sent before Generate`} onclick={() => run(useAs(shot, node))} data-testid="more-use">{as.label}</button>
		{/if}
		<button type="button" disabled={rejected && !picked} title={rejected && !picked ? `${name} is rejected – it stays out of Compare (X brings it back)` : picked ? 'Out of the selection (Ctrl + click does the same)' : 'Into the selection, A–D (Ctrl + click does the same) – Compare comes in stage 4'} onclick={() => { toggleSelected({ shot, key: node.key }, node); refocus(); }} data-testid="more-compare">{picked ? 'Remove from Compare' : 'Add to Compare'}</button>
		<button type="button" disabled={!url} title={url ? `${url.split('?')[0]} in a new tab` : `${name}: its file is gone`} onclick={() => { window.open(url, '_blank', 'noopener'); refocus(); }} data-testid="more-file">Show the file</button>
	</Menu>
</span>

<style>
	/* the Menu's trigger as one of the group's square buttons (tools.css .sc-seg > button.is-square) – pressed while open */
	.more {
		display: inline-flex;
	}
	.more :global(.sc-menu > .trigger) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: calc(var(--control-height) - 2 * var(--border-width-regular));
		min-height: var(--control-height);
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
	/* the items keep their words on one line */
	.more :global(.popup > button) {
		white-space: nowrap;
	}
</style>
