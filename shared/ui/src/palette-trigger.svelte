<script lang="ts">
	/**
	 * The top bar's button that opens the `CommandPalette` (admin, creator, embassy): search icon, the label and the
	 * shortcut hint in a 2px ink frame. Below 768px it collapses to the icon (the label stays the accessible name and
	 * tooltip). `compact` never shows the label text (icon + kbd), for bars that need the room (creator breadcrumb,
	 * embassy nav).
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
		/** icon + kbd only (no label text) at every width */
		compact?: boolean;
		onclick: () => void;
	} = $props();
</script>

<button type="button" class="sc-palette-trigger" class:compact aria-label={label} title={label} data-testid="palette-trigger" {onclick}>
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
	.compact {
		flex: none;
		min-width: 0;
	}
	.compact .wide-only {
		display: none;
	}
	@media (max-width: 767px) {
		.sc-palette-trigger {
			justify-content: center;
			min-width: var(--control-height);
		}
		.wide-only,
		kbd {
			display: none;
		}
	}
</style>
