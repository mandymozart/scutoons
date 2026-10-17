<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * A sticky action bar at the bottom of a scrolling panel (owner 2026-10-16: the item actions "stuck to the bottom",
	 * what's behind "scrolls under them in a gradient fading to transparent … without a hard line"): put it last in the
	 * scroll box; the content scrolls under it and fades out into the panel's background (`--paper` – black inside a
	 * dark scope). No rule on top. Its buttons share the row (flex, wrapping), the primary one last.
	 *
	 * The scroll box's bottom padding (incl. the safe area) belongs to the bar: set `--sticky-actions-inset` on the scroll
	 * box to that padding, the bar then reaches the box's bottom edge and keeps the padding inside (DESIGN.md
	 * "Sticky action bar").
	 */
	let { label, testid, children }: { label?: string; testid?: string; children: Snippet } = $props();
</script>

<div class="sc-sticky-actions" role="group" aria-label={label} data-testid={testid}>
	{@render children()}
</div>

<style>
	.sc-sticky-actions {
		--inset: var(--sticky-actions-inset, 0rem);
		position: sticky;
		bottom: calc(-1 * var(--inset));
		z-index: 1;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		/* the fade: from transparent (the content above shows) into the background behind the buttons */
		margin: 0 0 calc(-1 * var(--inset));
		padding: var(--space-3) 0 var(--inset);
		margin-top: calc(-1 * var(--space-15));
		/* eased, so the content fades out softly over the whole 2.5rem instead of going dark at once */
		background: linear-gradient(
			to bottom,
			transparent,
			color-mix(in srgb, var(--paper) 25%, transparent) calc(0.35 * (var(--space-3) - var(--space-05))),
			color-mix(in srgb, var(--paper) 70%, transparent) calc(0.7 * (var(--space-3) - var(--space-05))),
			var(--paper) calc(var(--space-3) - var(--space-05))
		);
		pointer-events: none;
	}

	.sc-sticky-actions > :global(*) {
		pointer-events: auto;
	}

	.sc-sticky-actions > :global(button),
	.sc-sticky-actions > :global(.btn),
	.sc-sticky-actions > :global(a) {
		flex: 1 1 auto;
	}

	/* round icon buttons keep their circle; the worded ones share the rest of the row */
	.sc-sticky-actions > :global(.sc-icon-button) {
		flex: none;
	}
</style>
