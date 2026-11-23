<script lang="ts">
	import Icon from '../icon.svelte';

	/**
	 * A chip that is on or off (deck-kit controls.rs `chips`: a filter's facts): a tree row less 4px high, a control's
	 * corner, the small type; on, the accent ground with the surface's ink; off, its line and muted words. Several side by
	 * side, each on its own (one-of-many is PillTabs).
	 */
	let { label, on = false, icon, title, disabled = false, onclick, testid }: { label: string; on?: boolean; icon?: string; title?: string; disabled?: boolean; onclick?: () => void; testid?: string } = $props();
</script>

<button type="button" class="kit-chip" class:on aria-pressed={on} {title} {disabled} {onclick} data-testid={testid}>{#if icon}<Icon name={icon} size="0.86rem" strokeWidth={2} />{/if}{label}</button>

<style>
	.kit-chip {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: calc(var(--editor-pad) / 2);
		box-sizing: border-box;
		height: calc(var(--editor-tree-row) - 4 * var(--editor-edge)); /* control-ok: deck-kit chips are a tree row less 4px */
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-muted);
		font: inherit;
		font-size: var(--editor-small);
		line-height: 1;
		white-space: nowrap;
		cursor: pointer;
	}
	.kit-chip :global(svg) {
		width: var(--editor-small);
		height: var(--editor-small);
	}
	.on {
		border-color: var(--tool-accent);
		background: var(--tool-accent);
		color: var(--tool-popover);
		text-shadow: none;
	}
	.on :global(svg) {
		filter: none;
	}
	@media (hover: hover) {
		.kit-chip:hover:not(:disabled, .on) {
			color: var(--tool-text);
			border-color: var(--tool-muted);
		}
	}
	.kit-chip:focus-visible {
		outline: 2px solid var(--tool-accent);
		outline-offset: 1px;
	}
	.kit-chip:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
