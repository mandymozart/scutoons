<script lang="ts">
	// A folded rejected card on the variants board (#125; VARIANTS-CANVAS §3.5 – owner 2026-10-20: "wenn rejected the need
	// to fold somehow so we can make space"): a STRIP in the card's place, drawn like the house Rail – the picture's height,
	// 1.5rem wide, a muted frame, the ✕ and the name muted, written upwards; "g3 +2" = its branch / its folded neighbours
	// folded in with it. A row of nothing but strips LIES DOWN (`lie`): one line. A click opens a LOOK (the strip unfolds
	// with its branch, muted, until the current card leaves it or Esc). It stays CURRENT after its X – X again brings back
	// what that X folded. An option of its band's listbox; the arrow keys skip it.
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { TOP, stripLabel, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';

	let { node, title, current, onpick }: { node: BoardNode; title: string; current: boolean; onpick: (e: MouseEvent) => void } = $props();
	const text = $derived(stripLabel(node));
</script>

<button type="button" class="strip" class:lie={node.lie} class:current data-key={node.key} data-keys={(node.keys ?? []).join(' ')} data-control-exempt="a strip" tabindex={current ? 0 : -1} role="option" aria-selected={current} aria-label={m['storyboard.board_strip_label']({ name: text })} {title} style:left="{node.x}rem" style:top="{node.y + TOP}rem" style:width="{node.w}rem" style:height="{node.h}rem" onclick={onpick} data-testid="strip-{node.key}">
	<span class="x"><Icon name="x" size="0.95rem" /></span><!-- close-ok: the rejected status, not a close --><span class="v">{text}</span>
</button>

<style>
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
		scroll-margin: 3.5rem 1rem 4rem;
	}
	.strip:hover {
		border-color: var(--ink);
	}
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
