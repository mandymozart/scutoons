<script lang="ts" module>
	import { panelWidth } from '@scutoons/ui';

	/** the Flow panel's width (shared by every level: one stored width, 28rem by default, drags wider up to half the window) */
	export const flowWidth = () => panelWidth('sc-panel-width:flow', 28);
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { pagePanel } from '$lib/ask.svelte.ts';

	/**
	 * A right-docked Flow panel for a page without its own panel frame (the toon page; the chapter editor and MetaPage dock
	 * theirs themselves): fixed left of the Ask panel, its left edge drags it wider. The PAGE makes room – it sets
	 * `margin-right: var(--panel-width)` from 768px with the same `size.css` (like the chapter editor's Flow and the meta
	 * pages' Problems / History), so nothing is hidden under the panel. With Ask open too the sidebar folds to its rail.
	 */
	let { size, children }: { size: ReturnType<typeof panelWidth>; children: Snippet } = $props();

	$effect(() => {
		pagePanel.open = true;
		return () => (pagePanel.open = false);
	});
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter): drag the panel wider -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" style:--panel-width={size.css} onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}></div>
<aside class="dock" style:--panel-width={size.css} data-testid="flow-dock">
	{@render children()}
</aside>

<style>
	.dock {
		position: fixed;
		top: var(--top, 3.5rem);
		/* left of the Ask panel: Ask belongs to the shell, the page's panels to the page */
		right: var(--ask-inset, 0rem);
		bottom: var(--action-bar-height, 0rem);
		width: var(--panel-width);
		max-width: 100vw;
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
		z-index: 20;
		box-sizing: border-box;
		overflow: hidden;
	}
	.resize {
		position: fixed;
		z-index: 21;
		top: var(--top, 3.5rem);
		bottom: var(--action-bar-height, 0rem);
		right: calc(var(--ask-inset, 0rem) + var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.resize:hover,
	.resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	@media (max-width: 767px) {
		.resize {
			display: none;
		}
	}
</style>
