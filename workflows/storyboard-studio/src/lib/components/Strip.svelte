<script>
	// A folded rejected card on the variants board (agents/research/VARIANTS-CANVAS.md §3.5 – owner 2026-10-20: "wenn
	// rejected the need to fold somehow so we can make space"): a STRIP in the card's place, drawn like the house Rail
	// (@scutoons/ui rail.svelte, board-sized) – the picture's height, 1.5rem wide, a muted frame, the ✕ and the id muted,
	// the id written upwards; "g3 +2" = its branch / its folded neighbours folded in with it (board-layout.js). A row of
	// nothing but strips LIES DOWN (`lie`): one line, the ✕ and the id across – the bottom region's rail. A click (Enter
	// when it is the current one) opens a LOOK (Board): the strip unfolds with its branch, muted, until the current card
	// leaves it or Esc. It stays CURRENT after its X (an ink frame) – X again brings back what that X folded. An option of
	// its band's listbox, as a card; the arrow keys skip it.
	import { Icon } from '@scutoons/ui';
	import { TOP, shortId, stripLabel } from '../board-layout.js';

	/** @type {{ node: any, title: string, current: boolean, onpick: (e: MouseEvent) => void }} */
	let { node, title, current, onpick } = $props();
	const text = $derived(stripLabel(node));
	/** its name in words: "g3 – rejected, 2 more folded in with it" ("+2" read out is no count) */
	const more = $derived(node.keys.length - 1);
	const name = $derived(`${shortId(node.record?.id ?? node.key)} – rejected${more ? `, ${more} more folded in with it` : ''}; Enter: a look into it, X: back`);
</script>

<button type="button" class="strip" class:lie={node.lie} class:current data-key={node.key} data-keys={node.keys.join(' ')} data-control-exempt="a strip" tabindex={current ? 0 : -1} role="option" aria-selected={current} aria-label={name} {title} style:left="{node.x}rem" style:top="{node.y + TOP}rem" style:width="{node.w}rem" style:height="{node.h}rem" onclick={onpick}>
	<span class="x"><Icon name="x" size="0.95rem" /></span><span class="v">{text}</span>
</button>

<style>
	/* the Rail's look, board-sized: no button inside (too narrow) – the strip is the button; zero-specificity buttons.css
	   undone (a strip is no control row) */
	.strip {
		position: absolute;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-05);
		box-sizing: border-box;
		min-height: 0;
		margin: 0;
		padding: var(--space-05) 0 0;
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: 0;
		background: var(--paper);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: 1;
		overflow: hidden;
		cursor: pointer;
		/* the board's keys bring it into view below the sticky band head */
		scroll-margin: 3.5rem 1rem 4rem;
	}
	.strip:hover {
		border-color: var(--ink);
	}
	/* the current one (its X just folded it): an ink frame, twice – as the current card */
	.strip.current {
		border-color: var(--ink);
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: var(--border-width-regular);
	}
	.x {
		display: flex;
		flex: none;
	}
	.v {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		white-space: nowrap;
	}
	/* lying down: one line, the ✕ and the id across */
	.strip.lie {
		flex-direction: row;
		align-items: center;
		gap: var(--space-025);
		padding: 0 var(--space-025);
	}
	.strip.lie .v {
		writing-mode: horizontal-tb;
		transform: none;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
