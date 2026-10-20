<script>
	// Compare – the board's split (agents/research/VARIANTS-CANVAS.md §3.4, issue #62 stage 4; the mockup
	// research/studio-layout/variants-canvas.html): right of the board, a Divider between (Board – its width remembered).
	// Its head: "Compare · n" and ✕ (round; Esc and C close too). The SIDES = the board's selection (A–D, A the reference
	// – compare.svelte.js), side by side at ONE height, each in its own shot's aspect – from any band – as large as the
	// pane leaves them (compare.js fitSides: at 1440 beside the Generate panel they scale down; nothing scrolls sideways).
	// An empty side (fewer than two, or one taken out – X, Ctrl + click: its letter stays free) is a dashed slot the next
	// click on a card fills (the first one). The videos on ONE clock (compare-clock.svelte.js – one Play, one scrubber:
	// CompareTransport; Space plays / pauses while the focus is in Compare – or on the board: `space`); images just
	// stand. At most 4 <video> elements, only while Compare is open – each lets go of its media when its side goes or
	// Compare closes.
	import { onDestroy } from 'svelte';
	import { CloseIcon, Icon, IconButton } from '@scutoons/ui';
	import { ratioOf } from '../board-layout.js';
	import { remPx } from '../board-view.svelte.js';
	import { fitSides, GAP, LETTERS, MIN_COL, MIN_H, PAD, SIDE_GAP, SIDE_HEAD, SIDE_INFO } from '../compare.js';
	import { createClock } from '../compare-clock.svelte.js';
	import { find, studio } from '../studio.svelte.js';
	import CompareSide from './CompareSide.svelte';
	import CompareTransport from './CompareTransport.svelte';

	/** @type {{ sides: ({ shot: string, key: string, node: any, takes: Map<string, any> } | null)[], width: string, onclose: () => void }} */
	let { sides, width, onclose } = $props();
	const clock = createClock();
	onDestroy(() => clock.stop());
	let pane = $state();
	/** Board: Space on the board while Compare is open – the clock plays / pauses */
	export const space = () => clock.length && clock.toggle();
	/** Board: opened by its button – the focus into the pane (Space, Esc, C are its keys then) */
	export const focus = () => pane?.focus({ preventScroll: true });

	/** the sides' room (px – measured; rem below) */
	let bw = $state(0);
	let bh = $state(0);
	const aspectOf = (shot) => find(shot)?.shot?.aspect;
	/** the sides and the empty ones (null – at least two places; an empty one in the current shot's aspect) */
	const slots = $derived(sides.length >= 2 ? sides : [...sides, ...Array(2 - sides.length).fill(null)]);
	const hole = $derived(slots.indexOf(null));
	const ratios = $derived(slots.map((s) => ratioOf(s ? aspectOf(s.shot) : studio.sel?.shot?.aspect)));
	// a very short pane: the pictures at least MIN_H high (it scrolls down then – never sideways: the width still fits)
	const fit = $derived.by(() => {
		const rem = remPx();
		const height = Math.max(MIN_H, bh / rem - 2 * PAD - SIDE_HEAD - SIDE_INFO - 2 * SIDE_GAP);
		return fitSides({ ratios, width: bw / rem - 2 * PAD, height, gap: GAP, minCol: MIN_COL });
	});
	const videos = $derived(sides.some((s) => s?.node?.record?.kind === 'video' || s?.node?.source === 'take'));

	function keys(e) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return;
		if (e.key === 'Escape' || e.key === 'c' || e.key === 'C') {
			if (e.repeat) return;
			onclose();
		} else if (e.key === ' ') {
			// a focused button's Space is its own click (Play, ✕)
			if (e.target.closest?.('button')) return;
			if (!e.repeat && clock.length) clock.toggle();
		} else return;
		e.preventDefault();
		e.stopPropagation();
	}
</script>

<!-- the pane takes the focus on a click inside it (tabindex −1): Space is Compare's then -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
<section class="compare" bind:this={pane} style:width aria-label="Compare – the selected cards side by side" tabindex="-1" onkeydown={keys} data-testid="compare">
	<header class="head">
		<Icon name="columns" />
		<h2>Compare · {sides.filter(Boolean).length}</h2>
		<IconButton class="sc-round" label="Close Compare (Esc)" onclick={onclose} data-testid="compare-close"><CloseIcon /></IconButton>
	</header>
	<div class="sides" bind:clientWidth={bw} bind:clientHeight={bh} style:--pad="{PAD}rem" style:--gap="{GAP}rem" style:--side-gap="{SIDE_GAP}rem">
		<div class="row">
			{#each slots as side, i (side?.key ?? `empty-${i}`)}
				{@const r = ratios[i]}
				{#if side}
					<CompareSide {side} letter={LETTERS[i]} h={fit.h} pw={fit.h * r} w={fit.widths[i]} other={side.shot !== studio.sel?.id} {clock} />
				{:else}
					<div class="slot" style:width="{fit.widths[i]}rem" style:padding-top="{SIDE_HEAD + SIDE_GAP}rem" data-side={LETTERS[i]}>
						<span class="hole" style:width="{fit.h * r}rem" style:height="{fit.h}rem">{i === hole ? `Click a card – it comes in as ${LETTERS[i]}` : `${LETTERS[i]} – empty`}</span>
					</div>
				{/if}
			{/each}
		</div>
	</div>
	{#if videos}<CompareTransport {clock} />{/if}
</section>

<style>
	.compare {
		display: flex;
		flex: none;
		flex-direction: column;
		box-sizing: border-box;
		min-width: 0;
		min-height: 0;
		border-left: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		outline: none;
	}
	/* as a panel's head: one control row, the label style, a rule under it */
	.head {
		display: flex;
		flex: none;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-025) var(--space-05) var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		flex: 1;
		min-width: 0;
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* the sides' room: never sideways – only a pane too short for the smallest picture scrolls down */
	.sides {
		flex: 1;
		min-height: 0;
		overflow: hidden auto;
		scrollbar-width: thin;
	}
	.row {
		display: flex;
		align-items: flex-start;
		gap: var(--gap);
		padding: var(--pad);
	}
	.slot {
		flex: none;
		min-width: 0;
	}
	/* the empty side: a dashed slot (a slot is a dashed frame – DESIGN "Media fields are slots") */
	.hole {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		padding: var(--space-05);
		border: var(--border-width-regular) dashed var(--ink);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		text-align: center;
	}
</style>
