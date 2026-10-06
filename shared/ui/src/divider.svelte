<script lang="ts">
	/**
	 * The ONE drag edge between two docked regions (agents/DOCKING.md): a `role="separator"` on a region's inner border
	 * (WAI-ARIA window splitter). Drag = resize, ← / → = 16 px (Shift 64) towards the region's growing side, Home or a
	 * double-click = the normal width; the size object (`resizable`, `dockRegion`, `panelWidth`) remembers it. 2px ink
	 * border at rest, a 3px ink line on hover / focus; hidden below 768px (regions become drawers / sheets there).
	 *
	 * Placed in its region's positioned container: `edge="right"` for a region docked on the left (`at` = the region's
	 * width from the container's left), `edge="left"` for one on the right (`at` from the container's right). `fixed`:
	 * relative to the viewport between `--sc-divider-top` and `--sc-divider-bottom` (a fixed panel such as Ask).
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
		label,
		controls,
		fixed = false,
		class: className = ''
	}: {
		size: Size;
		edge: 'left' | 'right';
		/** CSS length: where the region's inner border is (its width, plus whatever is docked further out) */
		at: string;
		label: string;
		/** id of the region it resizes */
		controls?: string;
		fixed?: boolean;
		class?: string;
	} = $props();
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	class="sc-divider edge-{edge} {className}"
	class:fixed
	role="separator"
	aria-orientation="vertical"
	aria-label={label}
	aria-controls={controls}
	aria-valuenow={size.width ?? undefined}
	tabindex="0"
	title={label}
	style:--sc-divider-at={at}
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
	.fixed {
		position: fixed;
		z-index: 31;
		top: var(--sc-divider-top, 0rem);
		bottom: var(--sc-divider-bottom, 0rem);
	}
	.sc-divider:hover,
	.sc-divider:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	:global(html.sc-resizing),
	:global(html.sc-resizing *) {
		cursor: col-resize !important;
		user-select: none !important;
	}
	@media (max-width: 767px) {
		.sc-divider {
			display: none;
		}
	}
</style>
