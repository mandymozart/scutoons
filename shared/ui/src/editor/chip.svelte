<script lang="ts">
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A chip – the kit's badge (deck-kit controls.rs `chips`: a filter's facts): small and delicate (RULES H13), a tree row
	 * less 8px high, round, the label type; off, its line and muted words; on, the text colour as line and words on a faint
	 * ground – no hue. `tone` makes it a status badge, the only chips with a hue (ok · warning · error); without `onclick`
	 * it is a plain mark. The states' colours are kit.css's `--kit-chip-*`.
	 */
	type Props = {
		label: string;
		on?: boolean;
		icon?: string;
		tone?: 'ok' | 'warning' | 'error';
		title?: string;
		disabled?: boolean;
		onclick?: () => void;
		testid?: string;
	} & Omit<HTMLButtonAttributes, 'onclick' | 'title'>;

	let { label, on = false, icon, tone, title, disabled = false, onclick, testid, ...rest }: Props = $props();
</script>

{#snippet inside()}{#if icon}<Icon name={icon} size="0.72rem" strokeWidth={2} />{/if}{label}{/snippet}

{#if onclick}
	<button type="button" {...rest} class="kit-chip {tone ?? ''} {rest.class ?? ''}" class:on aria-pressed={tone ? undefined : on} {title} {disabled} {onclick} data-testid={testid}>{@render inside()}</button>
{:else}
	<span {...rest as Record<string, unknown>} class="kit-chip mark {tone ?? ''} {rest.class ?? ''}" class:on {title} data-testid={testid}>{@render inside()}</span>
{/if}

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
		border: var(--editor-edge) solid var(--kit-chip-rest-border);
		border-radius: calc(var(--chip) / 2);
		background: var(--kit-chip-rest-ground);
		color: var(--kit-chip-rest-text);
		font: inherit;
		font-size: var(--editor-label);
		line-height: 1;
		text-shadow: none;
		white-space: nowrap;
		cursor: pointer;
	}
	.mark {
		cursor: default;
	}
	.kit-chip :global(svg) {
		width: var(--editor-label);
		height: var(--editor-label);
	}
	.kit-chip[data-kit-state~='hover'] {
		border-color: var(--kit-chip-hover-border);
		color: var(--kit-chip-hover-text);
	}
	@media (hover: hover) {
		button.kit-chip:hover:not(:disabled, .on, .ok, .warning, .error) {
			border-color: var(--kit-chip-hover-border);
			color: var(--kit-chip-hover-text);
		}
	}
	.on {
		border-color: var(--kit-chip-on-border);
		background: var(--kit-chip-on-ground);
		color: var(--kit-chip-on-text);
	}
	.ok {
		border-color: var(--kit-chip-ok-border);
		background: var(--kit-chip-ok-ground);
		color: var(--kit-chip-ok-text);
	}
	.warning {
		border-color: var(--kit-chip-warning-border);
		background: var(--kit-chip-warning-ground);
		color: var(--kit-chip-warning-text);
	}
	.error {
		border-color: var(--kit-chip-error-border);
		background: var(--kit-chip-error-ground);
		color: var(--kit-chip-error-text);
	}
	.kit-chip:focus-visible,
	.kit-chip[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.kit-chip:disabled {
		opacity: var(--kit-chip-disabled-opacity);
		cursor: not-allowed;
	}
</style>
