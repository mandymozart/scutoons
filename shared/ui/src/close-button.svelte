<script lang="ts">
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import CloseIcon from './close-icon.svelte';

	/**
	 * The close / remove control of dialogs, sheets, panels, toasts (agents/DESIGN.md "Close cross"): a frameless
	 * --control-height square (>= 44px on touch) with the one drawn `CloseIcon` centred, level with the headline.
	 * `label` is required (aria-label + title). `size` = the adjacent headline's font-size, `stroke` 2 (default) or 1.
	 */
	type Props = {
		label: string;
		size?: string;
		stroke?: 1 | 2;
	} & Omit<HTMLButtonAttributes, 'children'>;

	let { label, size, stroke = 2, type = 'button', ...rest }: Props = $props();
</script>

<button {type} aria-label={label} title={label} {...rest} class="sc-close-button {rest.class ?? ''}">
	<CloseIcon {size} {stroke} />
</button>

<style>
	.sc-close-button {
		display: inline-grid;
		place-items: center;
		flex: none;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		margin: 0;
		padding: 0;
		background: transparent;
		border: var(--border-width-regular) solid transparent;
		border-radius: var(--radius-none);
		color: var(--ink);
		cursor: pointer;
		touch-action: manipulation;
	}
	.sc-close-button:hover:not(:disabled) {
		border-color: var(--border-muted);
	}
	.sc-close-button:focus-visible {
		outline: var(--border-width-focus, 3px) solid var(--ink);
		outline-offset: var(--focus-offset, 2px);
	}
	.sc-close-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
