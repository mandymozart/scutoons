<script lang="ts">
	/**
	 * The top bar's button that opens the `CommandPalette` (admin, creator, embassy): search icon, the label and the
	 * shortcut hint in a 2px ink frame (a text control: square). Below 768px, and always when `compact`, it is the
	 * icon alone – a standalone icon-only button, so round like `.sc-round` (owner 2026-10-07, DESIGN.md "Round vs
	 * square"); the label + shortcut stay the accessible name and tooltip.
	 * Spacing to its neighbours is the bar's business.
	 */
	import Icon from './icon.svelte';

	let {
		label,
		shortcut = 'Ctrl K',
		compact = false,
		onclick
	}: {
		/** "Search or jump to …" – visible from 768px, always the accessible name and tooltip */
		label: string;
		/** the hotkey hint shown in the `kbd` (hidden below 768px) */
		shortcut?: string;
		/** the round search icon at every width (label + shortcut only in the tooltip) */
		compact?: boolean;
		onclick: () => void;
	} = $props();
</script>

<button type="button" class="sc-palette-trigger" class:compact aria-label={label} title="{label} ({shortcut})" data-testid="palette-trigger" {onclick}>
	<span class="search-label"><Icon name="search" /> <span class="wide-only">{label}</span></span><kbd>{shortcut}</kbd>
</button>

<style>
	.sc-palette-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		box-sizing: border-box;
		min-width: 14rem;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		background: var(--paper);
		color: var(--text-muted);
		font: inherit;
		white-space: nowrap;
		cursor: pointer;
	}
	.sc-palette-trigger:hover {
		color: var(--ink);
	}
	.search-label {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}
	kbd {
		font: inherit;
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		border: var(--border-width-regular) solid var(--ink);
		color: var(--ink);
	}
	/* icon only = round (tools.css .sc-round): a --control-height circle, ink, inverted on hover */
	.compact {
		flex: none;
		justify-content: center;
		width: var(--control-height);
		min-width: var(--control-height);
		padding: 0;
		border-radius: 50%;
		color: var(--ink);
	}
	.compact:hover,
	.compact:focus-visible {
		background: var(--ink);
		color: var(--paper);
	}
	.compact .wide-only,
	.compact kbd {
		display: none;
	}
	@media (max-width: 767px) {
		.sc-palette-trigger {
			flex: none;
			justify-content: center;
			width: var(--control-height);
			min-width: var(--control-height);
			padding: 0;
			border-radius: 50%;
			color: var(--ink);
		}
		.sc-palette-trigger:hover,
		.sc-palette-trigger:focus-visible {
			background: var(--ink);
			color: var(--paper);
		}
		.wide-only,
		kbd {
			display: none;
		}
	}
</style>
