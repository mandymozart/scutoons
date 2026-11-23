<script lang="ts">
	import Icon from '../icon.svelte';

	/**
	 * A chip – the kit's badge – that is on or off (deck-kit controls.rs `chips`: a filter's facts): small and delicate
	 * (RULES H13, owner 2026-11-24, #841): a tree row less 8px high, round, the label type; on, the accent as line and words
	 * on a faint accent ground (never a solid blue fill); off, its line and muted words. Several side by side, each on its
	 * own (one-of-many is PillTabs).
	 */
	let { label, on = false, icon, title, disabled = false, onclick, testid }: { label: string; on?: boolean; icon?: string; title?: string; disabled?: boolean; onclick?: () => void; testid?: string } = $props();
</script>

<button type="button" class="kit-chip" class:on aria-pressed={on} {title} {disabled} {onclick} data-testid={testid}>{#if icon}<Icon name={icon} size="0.72rem" strokeWidth={2} />{/if}{label}</button>

<style>
	.kit-chip {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: calc(var(--editor-pad) / 2);
		box-sizing: border-box;
		--chip: calc(var(--editor-tree-row) - 8 * var(--editor-edge));
		height: var(--chip); /* control-ok: the kit's badge, a tree row less 8px (#841) */
		margin: 0;
		padding: 0 calc(var(--editor-pad) * 1.25);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: calc(var(--chip) / 2);
		background: none;
		color: var(--tool-muted);
		font: inherit;
		font-size: var(--editor-label);
		line-height: 1;
		white-space: nowrap;
		cursor: pointer;
	}
	.kit-chip :global(svg) {
		width: var(--editor-label);
		height: var(--editor-label);
	}
	.on {
		border-color: var(--tool-accent);
		background: color-mix(in srgb, var(--tool-accent) 14%, transparent);
		color: var(--tool-accent);
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
