<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';

	/**
	 * Fixed bottom action bar (PLATFORM.md "Admin UI patterns" → "Bottom action bar", owner rule for admin + creator):
	 * mirrors the top bar – same height (`--action-bar-height`, default 3.25rem), paper, 2px ink rule on top, full width,
	 * always visible (no fade, no auto-hide). `start` = pagination / filters / search / status, `secondary` = secondary
	 * actions, `end` = the primary action(s), right-most. Narrower than `breakpoint`: `end` stays visible, `start` and
	 * `secondary` move into a "More" panel that opens upwards. While a bar is on the page, `body` gets a bottom padding
	 * of the bar's height, so nothing hides behind it (place the component anywhere in the page markup).
	 */
	let {
		start,
		secondary,
		end,
		label = 'Page actions',
		moreLabel = 'More',
		breakpoint = 1024
	}: {
		start?: Snippet;
		secondary?: Snippet;
		end?: Snippet;
		/** accessible name of the bar */
		label?: string;
		moreLabel?: string;
		breakpoint?: number;
	} = $props();

	let narrow = $state(false);
	let moreOpen = $state(false);
	const uid = $props.id();

	$effect(() => {
		const query = window.matchMedia(`(max-width: ${breakpoint - 1}px)`);
		narrow = query.matches;
		const change = (event: MediaQueryListEvent) => {
			narrow = event.matches;
			if (!narrow) moreOpen = false;
		};
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});

	const hasMore = $derived(!!start || !!secondary);
</script>

<div class="sc-action-bar" role="region" aria-label={label}>
	{#if narrow}
		{#if hasMore}
			<div class="more" {@attach clickOutside(() => (moreOpen = false))}>
				<button
					type="button"
					class="more-toggle"
					aria-expanded={moreOpen}
					aria-controls="{uid}-more"
					onclick={() => (moreOpen = !moreOpen)}
				>
					{moreLabel} <span aria-hidden="true">{moreOpen ? '▾' : '▴'}</span>
				</button>
				{#if moreOpen}
					<div class="more-panel" id="{uid}-more">
						{#if start}<div class="more-start">{@render start()}</div>{/if}
						{#if secondary}<div class="more-secondary">{@render secondary()}</div>{/if}
					</div>
				{/if}
			</div>
		{/if}
		<div class="end">{#if end}{@render end()}{/if}</div>
	{:else}
		<div class="start">{#if start}{@render start()}{/if}</div>
		<div class="end">
			{#if secondary}{@render secondary()}{/if}
			{#if end}{@render end()}{/if}
		</div>
	{/if}
</div>

<style>
	/* the page gets the bottom padding wherever the bar is placed in the markup */
	:global(body:has(.sc-action-bar)) {
		padding-bottom: calc(var(--action-bar-height, 3.25rem) + var(--space-1));
	}
	.sc-action-bar {
		position: fixed;
		z-index: 40;
		left: 0;
		right: 0;
		bottom: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		box-sizing: border-box;
		height: var(--action-bar-height, 3.25rem);
		padding: 0 var(--space-1);
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
		color: var(--ink);
	}
	.start {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: thin;
	}
	.end {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		margin-left: auto;
	}
	.more {
		position: relative;
	}
	.more-toggle {
		padding: 0.45em 1em;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.more-toggle:hover,
	.more-toggle[aria-expanded='true'] {
		border-color: var(--ink);
	}
	.more-panel {
		position: absolute;
		left: calc(-1 * var(--space-1));
		bottom: calc(100% + var(--space-05) + var(--border-width-regular));
		width: calc(100vw - var(--space-1) * 2);
		max-height: 70dvh;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		box-sizing: border-box;
		margin-left: var(--space-1);
		padding: var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.more-start,
	.more-secondary {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-075);
	}
</style>
