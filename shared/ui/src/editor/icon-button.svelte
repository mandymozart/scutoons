<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A lone icon you press (deck-kit controls.rs `icon_button`; in a pill bar.rs `inner_button`): square, its ground shows
	 * under the pointer, open (`aria-expanded`) and `pressed` take the pressed ground. Its side and corner are where it
	 * sits (--kit-h / --kit-r): in a Pill it is round, its corners sharing the pill's centre; in a panel a control's corner.
	 * `label` is its name and tooltip; `dot` marks it (Publish: something is not live); `tone` colours the icon.
	 */
	type Props = {
		label: string;
		icon?: string;
		pressed?: boolean;
		dot?: boolean;
		tone?: 'text' | 'muted' | 'go' | 'stop' | 'danger';
		href?: string;
		children?: Snippet;
	} & Omit<HTMLButtonAttributes, 'children'> &
		Omit<HTMLAnchorAttributes, 'children'>;

	let { label, icon, pressed, dot = false, tone = 'text', href, type = 'button', title, children, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if icon}<Icon name={icon} size="1.15rem" strokeWidth={2} />{:else}{@render children?.()}{/if}{#if dot}<span class="dot" aria-hidden="true"></span>{/if}
{/snippet}

{#if href}
	<a {href} aria-label={label} title={title ?? label} {...rest as HTMLAnchorAttributes} class="kit-icon-button {tone} {rest.class ?? ''}" class:pressed>{@render inside()}</a>
{:else}
	<button {type} aria-label={label} title={title ?? label} aria-pressed={pressed} {...rest as HTMLButtonAttributes} class="kit-icon-button {tone} {rest.class ?? ''}" class:pressed>{@render inside()}</button>
{/if}

<style>
	.kit-icon-button {
		position: relative;
		display: inline-grid;
		flex: none;
		place-items: center;
		box-sizing: border-box;
		width: var(--kit-h, var(--control-height));
		height: var(--kit-h, var(--control-height));
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: var(--kit-r, var(--editor-radius));
		background: none;
		color: var(--tool-text);
		text-decoration: none;
		cursor: pointer;
		/* a lone icon is 0.6 of its square (controls.rs); in a pill bar.rs's 1.15rem (the pill sets it) */
		--kit-icon-size: var(--kit-icon-in, calc(var(--kit-h, var(--control-height)) * 0.6));
	}
	.kit-icon-button :global(svg) {
		width: var(--kit-icon-size);
		height: var(--kit-icon-size);
	}
	.muted {
		color: var(--tool-muted);
	}
	.go {
		color: var(--tool-go);
	}
	.stop {
		color: var(--tool-stop);
	}
	.danger {
		color: var(--tool-danger);
	}
	.pressed,
	.kit-icon-button[aria-expanded='true'] {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		text-shadow: none;
	}
	.pressed :global(svg),
	.kit-icon-button[aria-expanded='true'] :global(svg) {
		filter: none;
	}
	@media (hover: hover) {
		.kit-icon-button:hover:not(:disabled, .pressed, [aria-expanded='true']) {
			background: var(--tool-hover);
		}
	}
	.kit-icon-button:focus-visible {
		outline: 2px solid var(--tool-accent);
		outline-offset: -2px;
	}
	.kit-icon-button:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
	.dot {
		position: absolute;
		top: 18%;
		right: 18%;
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background: var(--tool-warn);
	}
</style>
