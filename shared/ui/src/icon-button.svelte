<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	/**
	 * DESIGN.md IconButton: a --control-height circle (one control height, owner 2026-10-02), paper bg, 2px ink border, hover (and `pressed`) fill primary-cyan,
	 * one 16px black icon. `label` is required (aria-label + title). Icon = `icon` (image URL) or `children`.
	 */
	type Props = {
		label: string;
		icon?: string;
		pressed?: boolean;
		children?: Snippet;
	} & Omit<HTMLButtonAttributes, 'children'>;

	let { label, icon, pressed, type = 'button', children, ...rest }: Props = $props();
</script>

<button
	{type}
	aria-label={label}
	title={label}
	aria-pressed={pressed}
	{...rest}
	class="sc-icon-button {rest.class ?? ''}"
	class:pressed
>
	{#if icon}
		<img src={icon} alt="" width="16" height="16" />
	{:else}
		{@render children?.()}
	{/if}
</button>

<style>
	.sc-icon-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		margin: 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-full);
		color: var(--ink);
		cursor: pointer;
		touch-action: manipulation;
		transition: background-color var(--duration-fast) var(--easing);
	}
	.sc-icon-button:hover:not(:disabled),
	.pressed {
		background: var(--primary-cyan);
	}
	.sc-icon-button:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
	.sc-icon-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	img {
		display: block;
		width: 16px;
		height: 16px;
	}
</style>
