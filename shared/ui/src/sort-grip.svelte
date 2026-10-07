<script lang="ts">
	/**
	 * The drag grip ⋮⋮ at the start of a reorderable row (`sortable`, BACKLOG 10.53, DESIGN.md "Reorderable lists"):
	 * drag it (a mouse right away, touch after a press-and-hold), or focus it and press ↑ / ↓ (Alt+↑ / ↓ on the row).
	 * Frameless (it belongs to the row, not a standalone button); six square dots – Feather has no grip, so it is drawn
	 * here (DESIGN.md "Icons": a missing icon lives in the shared library).
	 */
	let {
		label,
		title,
		disabled = false,
		testid
	}: {
		/** the accessible name, e.g. "Move Mia" */
		label: string;
		/** the tooltip; default the label (e.g. "Drag, or Alt+↑ / Alt+↓") */
		title?: string;
		disabled?: boolean;
		testid?: string;
	} = $props();
</script>

<button type="button" class="sc-sort-grip" data-sort-grip aria-label={label} title={title ?? label} {disabled} data-testid={testid}>
	<svg viewBox="0 0 7 12" aria-hidden="true" focusable="false" fill="currentColor">
		<rect x="0" y="0" width="2.5" height="2.5" /><rect x="4.5" y="0" width="2.5" height="2.5" />
		<rect x="0" y="4.75" width="2.5" height="2.5" /><rect x="4.5" y="4.75" width="2.5" height="2.5" />
		<rect x="0" y="9.5" width="2.5" height="2.5" /><rect x="4.5" y="9.5" width="2.5" height="2.5" />
	</svg>
</button>

<style>
	.sc-sort-grip {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		align-self: center;
		box-sizing: border-box;
		height: var(--control-height);
		min-height: 0;
		width: 1.5rem;
		min-width: 0;
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		background: transparent;
		/* the row's colour (an inverted, selected row too), a bit quieter than its text */
		color: inherit;
		cursor: grab;
		/* the list still scrolls under a finger: a drag needs the press-and-hold */
		touch-action: manipulation;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	.sc-sort-grip svg {
		width: 0.5rem;
		height: 0.875rem;
		opacity: 0.6;
		pointer-events: none;
	}
	.sc-sort-grip:hover:not(:disabled),
	.sc-sort-grip:focus-visible {
		color: inherit;
		background: transparent;
	}
	.sc-sort-grip:hover:not(:disabled) svg,
	.sc-sort-grip:focus-visible svg {
		opacity: 1;
	}
	.sc-sort-grip:focus-visible {
		outline: var(--border-width-focus) solid currentColor;
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.sc-sort-grip:disabled {
		cursor: default;
		opacity: 0.35;
	}
	/* touch screens (BACKLOG 10.50 d): a target as wide as it is high (≥ 2.75rem) – the drawn dots stay small */
	@media (pointer: coarse) {
		.sc-sort-grip {
			width: var(--control-height);
		}
	}
	:global([data-sort-dragging]) .sc-sort-grip {
		cursor: grabbing;
	}
</style>
