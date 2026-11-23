<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * The kit's button (deck-kit controls.rs): words, an icon before them where it helps, one control high. `primary` is a
	 * view's one primary action (lime, its edge, ink words, an arrow out when it leaves for elsewhere – controls.rs
	 * `primary`), `framed` a control on the control ground with its line, `quiet` words alone with the hover ground under
	 * the pointer. `pressed` is the pressed segment; `fill` takes the room left, its words from the start. Height and corner are where it sits (--kit-h / --kit-r: a control's in
	 * a panel, the inside of a Pill less its inset in a Pill). `href` makes it a link with the same look.
	 */
	type Props = {
		variant?: 'primary' | 'framed' | 'quiet';
		icon?: string;
		outward?: boolean;
		pressed?: boolean;
		fill?: boolean;
		href?: string;
		disabled?: boolean;
		children: Snippet;
	} & Omit<HTMLButtonAttributes, 'children'> &
		Omit<HTMLAnchorAttributes, 'children'>;

	let { variant = 'framed', icon, outward = false, pressed, fill = false, href, disabled = false, type = 'button', children, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if icon}<Icon name={icon} size="1rem" strokeWidth={2} />{/if}<span class="words">{@render children()}</span>{#if outward}<Icon name="arrow-up-right" size="1rem" strokeWidth={2} />{/if}
{/snippet}

{#if href && !disabled}
	<a {href} {...rest as HTMLAnchorAttributes} class="kit-button {variant} {rest.class ?? ''}" class:pressed class:fill>{@render inside()}</a>
{:else}
	<button {type} {disabled} aria-pressed={pressed} {...rest as HTMLButtonAttributes} class="kit-button {variant} {rest.class ?? ''}" class:pressed class:fill>{@render inside()}</button>
{/if}

<style>
	.kit-button {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		gap: var(--kit-icon-gap);
		box-sizing: border-box;
		height: var(--kit-h, var(--control-height));
		margin: 0;
		padding: 0 calc(var(--editor-pad) * 1.5);
		border: var(--editor-edge) solid transparent;
		border-radius: var(--kit-r, var(--editor-radius));
		background: none;
		color: var(--tool-text);
		font: inherit;
		font-size: var(--kit-font, var(--editor-font));
		line-height: 1;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
	}
	.kit-button :global(svg) {
		flex: none;
		width: var(--kit-icon);
		height: var(--kit-icon);
	}
	.words {
		display: inline-flex;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* `fill`: it takes the room left, its words from the start (a status line) */
	.fill {
		flex: 1 1 auto;
		justify-content: flex-start;
		min-width: 0;
	}
	.framed {
		border-color: var(--tool-line);
		background: var(--tool-control);
		text-shadow: none;
	}
	.primary {
		border-color: var(--tool-primary-line);
		background: var(--tool-primary);
		color: var(--tool-primary-ink);
		text-shadow: none;
	}
	.primary :global(svg) {
		filter: none;
	}
	.pressed {
		border-color: transparent;
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		text-shadow: none;
	}
	@media (hover: hover) {
		.framed:hover:not(:disabled, .pressed),
		.quiet:hover:not(:disabled, .pressed) {
			background: var(--tool-hover);
		}
		.primary:hover:not(:disabled) {
			border-color: var(--tool-primary-ink);
		}
	}
	.kit-button:focus-visible {
		outline: 2px solid var(--tool-accent);
		outline-offset: 1px;
	}
	.kit-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
