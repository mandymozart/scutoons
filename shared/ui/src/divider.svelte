<script lang="ts">
	/**
	 * The ONE drag edge between two docked regions (agents/DOCKING.md): a `role="separator"` on a region's inner border
	 * (WAI-ARIA window splitter). Drag = resize, ← / → = 16 px (Shift 64) towards the region's growing side, Home or a
	 * double-click = the normal width; the size object (`resizable`, `dockRegion`, `panelWidth`, `panelHeight`)
	 * remembers it. 2px ink border at rest, a 3px ink line on hover / focus; hidden below 768px (regions become drawers /
	 * sheets there).
	 *
	 * Placed in its region's positioned container: `edge="right"` for a region docked on the left (`at` = the region's
	 * width from the container's left), `edge="left"` for one on the right (`at` from the container's right), `edge="top"`
	 * for a region along the bottom (the AppShell's bottom region: `at` = its height from the container's bottom, `from` =
	 * where it starts on the left; ↑ / ↓ instead of ← / →, a horizontal separator, the row-resize cursor). Every region is
	 * a tile of its container (AppShell, a page's own split) – nothing is fixed to the viewport.
	 */
	interface Size {
		readonly width: number | null;
		start(event: PointerEvent): void;
		key(event: KeyboardEvent): void;
		reset(): void;
	}
	let {
		size,
		edge,
		at,
		from = '0rem',
		label,
		controls,
		class: className = ''
	}: {
		size: Size;
		edge: 'left' | 'right' | 'top';
		/** CSS length: where the region's inner border is (its width, plus whatever is docked further out) */
		at: string;
		/** CSS length (edge "top"): the region's left start in the container */
		from?: string;
		label: string;
		/** id of the region it resizes */
		controls?: string;
		class?: string;
	} = $props();
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	class="sc-divider edge-{edge} {className}"
	role="separator"
	aria-orientation={edge === 'top' ? 'horizontal' : 'vertical'}
	aria-label={label}
	aria-controls={controls}
	aria-valuenow={size.width ?? undefined}
	tabindex="0"
	title={label}
	style:--sc-divider-at={at}
	style:--sc-divider-from={edge === 'top' ? from : null}
	onpointerdown={size.start}
	onkeydown={size.key}
	ondblclick={size.reset}
></div>

<style>
	.sc-divider {
		position: absolute;
		z-index: 15;
		top: 0;
		bottom: 0;
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.edge-right {
		left: calc(var(--sc-divider-at) - 0.375rem);
	}
	.edge-left {
		right: calc(var(--sc-divider-at) - 0.375rem);
	}
	.edge-top {
		top: auto;
		bottom: calc(var(--sc-divider-at) - 0.375rem);
		left: var(--sc-divider-from);
		right: 0;
		width: auto;
		height: 0.75rem;
		cursor: row-resize;
	}
	.sc-divider:hover,
	.sc-divider:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	.edge-top:hover,
	.edge-top:focus-visible {
		background-size: 100% var(--border-width-focus);
	}
	:global(html.sc-resizing),
	:global(html.sc-resizing *) {
		cursor: col-resize !important;
		user-select: none !important;
	}
	:global(html.sc-resizing.sc-resizing-rows),
	:global(html.sc-resizing.sc-resizing-rows *) {
		cursor: row-resize !important;
	}
	@media (max-width: 767px) {
		.sc-divider {
			display: none;
		}
	}
</style>
