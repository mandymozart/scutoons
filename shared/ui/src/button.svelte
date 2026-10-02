<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

	/**
	 * DESIGN.md Button. primary: ink bg, paper text · secondary: paper bg, 2px border-muted → ink on hover ·
	 * clear: no chrome. Square, sentence case, never uppercase. `href` renders a link with the same look.
	 * `size="sm"` is the compact variant for bars (creator, admin): the same 1rem type and height, less side padding
	 * (`space-15`). Every size is exactly `--control-height` (owner 2026-10-02, one control height).
	 * Everywhere the viewer's concept (owner 2026-10-01): no hover inversion, disabled at 0.5 opacity.
	 */
	type Props = {
		variant?: 'primary' | 'secondary' | 'clear';
		size?: 'md' | 'sm';
		href?: string;
		disabled?: boolean;
		children: Snippet;
	} & Omit<HTMLButtonAttributes, 'children'> &
		Omit<HTMLAnchorAttributes, 'children'>;

	let { variant = 'primary', size = 'md', href, disabled = false, type = 'button', children, ...rest }: Props =
		$props();
</script>

{#if href && !disabled}
	<a {href} {...rest as HTMLAnchorAttributes} class="sc-button {variant} {size} {rest.class ?? ''}">
		{@render children()}
	</a>
{:else}
	<button
		{type}
		{disabled}
		{...rest as HTMLButtonAttributes}
		class="sc-button {variant} {size} {rest.class ?? ''}"
		class:disabled
	>
		{@render children()}
	</button>
{/if}

<style>
	.sc-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-3);
		border: var(--border-width-regular) solid transparent;
		border-radius: var(--radius-none);
		font-family: var(--font-mono);
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
		text-align: center;
		text-decoration: none;
		text-transform: none;
		cursor: pointer;
		transition:
			border-color var(--duration-fast) var(--easing),
			background-color var(--duration-fast) var(--easing);
	}

	.primary {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}

	.secondary {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--border-muted);
	}
	.secondary:hover:not(.disabled),
	.secondary:focus-visible {
		border-color: var(--ink);
	}

	.clear {
		padding: 0;
		background: transparent;
		color: var(--ink);
		border-color: transparent;
	}
	.clear:hover:not(.disabled) {
		text-decoration: underline;
	}

	.sm {
		padding: 0 var(--space-15);
	}
	.sm.clear {
		padding: 0 var(--space-05);
	}

	.sc-button:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}

	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

</style>
