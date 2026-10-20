<script lang="ts">
	// Compare – the board's split (#125; VARIANTS-CANVAS §3.4): right of the bands, a Divider between (variants-canvas –
	// its width remembered). Its head: "Compare · n" and the close button (Esc and C close too). The SIDES = the board's
	// selection (A–D, A the reference), side by side at ONE height, each in its own shot's aspect – from any band – as
	// large as the pane leaves them (storyboard-compare.ts fitSides: nothing scrolls sideways). An empty side is a dashed
	// slot the next click on a card fills. The videos on ONE clock (storyboard-compare-clock.svelte.ts – one Play, one
	// scrubber: compare-transport; Space plays / pauses). At most 4 <video>, only while Compare is open.
	import { onDestroy } from 'svelte';
	import { CloseIcon, Icon, IconButton } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { ratioOf, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
	import { fitSides, GAP, LETTERS, MIN_COL, MIN_H, PAD, SIDE_GAP, SIDE_HEAD, SIDE_INFO } from '$lib/editor/storyboard-compare.ts';
	import { createClock } from '$lib/editor/storyboard-compare-clock.svelte.ts';
	import { remPx } from '$lib/editor/storyboard-board-view.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import CompareSide from '$lib/components/storyboard/compare-side.svelte';
	import CompareTransport from '$lib/components/storyboard/compare-transport.svelte';

	type SideIn = { shot: string; key: string; node: BoardNode | null; aspect: string | null } | null;
	let { sb, sides, width, onclose }: { sb: StoryboardPage; sides: SideIn[]; width: string; onclose: () => void } = $props();
	const clock = createClock();
	onDestroy(() => clock.stop());
	let pane = $state<HTMLElement>();
	/** Space on the board while Compare is open */
	export const space = () => {
		if (clock.length) clock.toggle();
	};
	export const focus = () => pane?.focus({ preventScroll: true });

	let bw = $state(0);
	let bh = $state(0);
	const slots = $derived(sides.length >= 2 ? sides : [...sides, ...Array<SideIn>(2 - sides.length).fill(null)]);
	const hole = $derived(slots.indexOf(null));
	const ratios = $derived(slots.map((s) => ratioOf(s ? s.aspect : sb.sel?.shot.aspect)));
	const fit = $derived.by(() => {
		const rem = remPx();
		const height = Math.max(MIN_H, bh / rem - 2 * PAD - SIDE_HEAD - SIDE_INFO - 2 * SIDE_GAP);
		return fitSides({ ratios, width: bw / rem - 2 * PAD, height, gap: GAP, minCol: MIN_COL });
	});
	const videos = $derived(sides.some((s) => s?.node?.record?.kind === 'video'));

	function keys(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return;
		if (e.key === 'Escape' || e.key === 'c' || e.key === 'C') {
			if (e.repeat) return;
			onclose();
		} else if (e.key === ' ') {
			if ((e.target as Element).closest?.('button')) return;
			if (!e.repeat && clock.length) clock.toggle();
		} else return;
		e.preventDefault();
		e.stopPropagation();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
<section class="compare" bind:this={pane} style:width aria-label={m['storyboard.compare_label']()} tabindex="-1" onkeydown={keys} data-testid="compare">
	<header class="head">
		<Icon name="columns" />
		<h2>{m['storyboard.compare_n']({ n: sides.filter(Boolean).length })}</h2>
		<IconButton class="sc-round" label={m['storyboard.compare_close']()} onclick={onclose} data-testid="compare-close"><CloseIcon /></IconButton>
	</header>
	<div class="sides" bind:clientWidth={bw} bind:clientHeight={bh} style:--pad="{PAD}rem" style:--gap="{GAP}rem">
		<div class="row">
			{#each slots as side, i (side?.key ?? `empty-${i}`)}
				{#if side && side.node}
					<CompareSide {sb} shot={side.shot} node={side.node} letter={LETTERS[i]} h={fit.h} pw={fit.h * ratios[i]} w={fit.widths[i]} other={side.shot !== sb.current} {clock} />
				{:else}
					<div class="slot" style:width="{fit.widths[i]}rem" style:padding-top="{SIDE_HEAD + SIDE_GAP}rem" data-side={LETTERS[i]}>
						<span class="hole" style:width="{fit.h * ratios[i]}rem" style:height="{fit.h}rem">{i === hole ? m['storyboard.compare_hole']({ letter: LETTERS[i] }) : m['storyboard.compare_empty']({ letter: LETTERS[i] })}</span>
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
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
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
