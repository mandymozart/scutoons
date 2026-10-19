<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import { shellDock } from './dock.svelte.ts';
	import type { PanelHeight } from './panel-width.svelte.ts';
	import Divider from './divider.svelte';
	import Rail from './rail.svelte';

	/**
	 * The AppShell's bottom region (app-shell.svelte – read its header; agents/DOCKING.md "Bottom region", owner 2026-10-19:
	 * the storyboard studio's timeline "underneath the other windows"), split out of the shell: the landmark around the
	 * page's `content` in the frame's second row (the shell's grid sizes the row and keeps the sidebar beside it; from
	 * 768px – phones: hidden, still mounted), its top edge a `Divider` (measured from the frame's bottom, up to half the
	 * frame), and its horizontal `Rail` – folded (`size.fold`) or while a region is maximized (`railed`). A rail, it stays
	 * mounted, laid out at its height, hidden. While it shows, the toasts sit above it (`--sc-shell-bottom-lift` on the root).
	 */
	let { content, size, railed, maximized, phone, frame, from, label, icon, resizeLabel, unfoldLabel }: {
		content: Snippet;
		/** its height and fold */
		size: PanelHeight;
		/** a rail: folded, or a region is maximized */
		railed: boolean;
		/** a region is maximized: its rail restores, no drag edge */
		maximized: boolean;
		phone: boolean;
		/** the shell's frame (its bottom = where the drag measures from) */
		frame: () => HTMLElement | undefined;
		/** CSS length: where the region starts (right of the sidebar) – its edge's left end */
		from: string;
		label: string;
		icon?: string;
		resizeLabel: string;
		unfoldLabel: string;
	} = $props();

	/** a horizontal rail is as thick as a vertical one is wide (= `--sc-shell-rail-width`, for the root's toast lift) */
	const RAIL = 'calc(var(--control-height) + 1rem + var(--border-width-regular))';
	// its drag measures from the frame's bottom and stops at half the frame (between the top bar and the ActionBar); a
	// remembered height taller than that shrinks with a smaller window
	$effect(() => {
		const s = size;
		s.edge = () => frame()?.getBoundingClientRect().bottom ?? window.innerHeight;
		s.room = () => (frame()?.clientHeight || window.innerHeight) / 2;
		const fit = () => s.fit();
		untrack(fit);
		window.addEventListener('resize', fit);
		return () => {
			s.edge = null;
			s.room = null;
			window.removeEventListener('resize', fit);
		};
	});
	// the toasts (ToastStack, fixed bottom-left) sit above the region while it shows (they covered the rail's button)
	$effect(() => {
		if (phone) return;
		const root = document.documentElement.style;
		root.setProperty('--sc-shell-bottom-lift', railed ? RAIL : size.css);
		return () => root.removeProperty('--sc-shell-bottom-lift');
	});
</script>

<section class="sc-shell-bottom" class:railed id="sc-shell-bottom" aria-label={label} data-testid="shell-bottom">
	{#if railed}
		<Rail side="bottom" {label} {icon} {unfoldLabel} controls="sc-shell-bottom" onunfold={() => (maximized ? shellDock.restore() : size.fold(false))} testid="bottom-rail" />
	{/if}
	<!-- it stays mounted while it is a rail: laid out at its height, hidden -->
	<div class="sc-shell-bottom-panel" class:kept={railed}>
		{@render content()}
	</div>
</section>
<!-- on its rail too: a drag that starts there only opens it (dockRegion); not while a region is maximized -->
{#if !maximized}
	<Divider {size} edge="top" at={railed ? 'var(--sc-shell-rail-width)' : size.css} {from} label={resizeLabel} controls="sc-shell-bottom" class="sc-shell-bottom-resize" />
{/if}

<style>
	/* a second row of the shell's frame, under the content and the dock (the shell's grid: its height, the sidebar beside it) */
	.sc-shell-bottom {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.sc-shell-bottom-panel {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
	}
	.sc-shell-bottom-panel > :global(*) {
		flex: 1 1 auto;
		min-height: 0;
	}
	/* its rail holds the content laid out at the region's height (inside its top border), hidden */
	.sc-shell-bottom-panel.kept {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: calc(var(--sc-shell-bottom-size) - var(--border-width-regular));
		visibility: hidden;
	}
	@media (min-width: 768px) {
		/* it spans the content and the dock; the dock's edges end on it */
		.sc-shell-bottom {
			grid-area: 2 / var(--sc-shell-content-col) / 3 / -1;
		}
	}
	@media (max-width: 767px) {
		/* phones: no bottom region (the fixed dock would cover it; the studio is a desktop tool) – hidden, still mounted */
		.sc-shell-bottom {
			display: none;
		}
	}
</style>
