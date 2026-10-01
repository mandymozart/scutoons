<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

	/**
	 * DESIGN.md Button. primary: ink bg, paper text · secondary: paper bg, 2px border-muted → ink on hover ·
	 * clear: no chrome. Square, sentence case, never uppercase. `href` renders a link with the same look.
	 * `size="sm"` is the tool variant (creator, admin): the same 1rem type, less padding, still a ≥ 44px hit target.
	 * In a tool (`.sc-tool`, tools.css) the secondary frame is ink and hover inverts (DESIGN.md "Brutalist tools").
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
		padding: var(--space-1) var(--space-3);
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
		border-color: var(--sc-button-secondary-border, var(--border-muted));
	}
	.secondary:hover:not(.disabled),
	.secondary:focus-visible {
		border-color: var(--ink);
	}
	/* tools: hover inverts (primary → paper, secondary → ink) */
	:global(.sc-tool) .primary:hover:not(.disabled) {
		background: var(--paper);
		color: var(--ink);
	}
	:global(.sc-tool) .secondary:hover:not(.disabled) {
		background: var(--ink);
		color: var(--paper);
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
		padding: var(--space-025) var(--space-1);
		line-height: 1.2;
	}
	.sm.clear {
		padding: var(--space-025) var(--space-05);
	}

	.sc-button:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}

	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	/* tools: disabled = the inactive frame, no see-through ink */
	:global(.sc-tool) .disabled {
		opacity: 1;
		background: var(--paper);
		color: var(--text-muted);
		border-color: var(--border-muted);
	}
</style>
