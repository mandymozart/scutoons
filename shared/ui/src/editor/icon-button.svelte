<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A lone icon you press (deck-kit controls.rs `icon_button`; in a pill bar.rs `inner_button`): square, its ground shows
	 * under the pointer, open (`aria-expanded`) and `pressed` take the pressed ground. Its side and corner are where it
	 * sits (--kit-h / --kit-r): in a Pill it is round, its corners sharing the pill's centre; standing alone it is round too (RULES H4, owner
	 * 2026-11-24: the deck's corner was a slip).
	 * `alone` gives a lone one (no pill round it) its line on the panel ground. `label` is its name and tooltip; `dot` marks
	 * it (Publish: something is not out yet); `tone` mutes it or says destroy (no hue for on: RULES H11). The states'
	 * colours are kit.css's `--kit-icon-button-*`.
	 */
	type Props = {
		label: string;
		icon?: string;
		pressed?: boolean;
		dot?: boolean;
		tone?: 'text' | 'muted' | 'stop' | 'danger';
		alone?: boolean;
		href?: string;
		children?: Snippet;
	} & Omit<HTMLButtonAttributes, 'children'> &
		Omit<HTMLAnchorAttributes, 'children'>;

	let { label, icon, pressed, dot = false, tone = 'text', alone = false, href, type = 'button', title, children, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if icon}<Icon name={icon} size="1.15rem" strokeWidth={2} />{:else}{@render children?.()}{/if}{#if dot}<span class="dot" aria-hidden="true"></span>{/if}
{/snippet}

{#if href}
	<a {href} aria-label={label} title={title ?? label} {...rest as HTMLAnchorAttributes} class="kit-icon-button {tone} {rest.class ?? ''}" class:pressed class:alone>{@render inside()}</a>
{:else}
	<button {type} aria-label={label} title={title ?? label} aria-pressed={pressed} {...rest as HTMLButtonAttributes} class="kit-icon-button {tone} {rest.class ?? ''}" class:pressed class:alone>{@render inside()}</button>
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
		border-radius: var(--kit-r, 50%);
		background: var(--kit-icon-button-rest-ground);
		color: var(--kit-icon-button-rest-text);
		text-decoration: none;
		cursor: pointer;
		/* a lone icon is 0.6 of its square (controls.rs); in a pill bar.rs's 1.15rem (the pill sets it) */
		--kit-icon-size: var(--kit-icon-in, calc(var(--kit-h, var(--control-height)) * 0.6));
	}
	.kit-icon-button :global(svg) {
		width: var(--kit-icon-size);
		height: var(--kit-icon-size);
	}
	/* alone (RULES H4): round, its line on the panel ground, the halo on its icon */
	.alone {
		border: var(--editor-edge) solid var(--kit-icon-button-alone-border);
		border-radius: 50%;
		background: var(--kit-icon-button-alone-ground);
		text-shadow: var(--tool-halo);
	}
	.alone :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.muted {
		color: var(--tool-muted);
	}
	.stop {
		color: var(--tool-stop);
	}
	.danger {
		color: var(--tool-danger);
	}
	.kit-icon-button[data-kit-state~='hover'] {
		background: var(--kit-icon-button-hover-ground);
	}
	@media (hover: hover) {
		.kit-icon-button:hover:not(:disabled, .pressed, [aria-expanded='true']) {
			background: var(--kit-icon-button-hover-ground);
		}
	}
	.pressed,
	.kit-icon-button[aria-expanded='true'] {
		background: var(--kit-icon-button-on-ground);
		color: var(--kit-icon-button-on-text);
		text-shadow: none;
	}
	.pressed :global(svg),
	.kit-icon-button[aria-expanded='true'] :global(svg) {
		filter: none;
	}
	.kit-icon-button:focus-visible,
	.kit-icon-button[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	.alone:focus-visible,
	.alone[data-kit-state~='focus'] {
		outline-offset: 1px;
	}
	.kit-icon-button:disabled {
		opacity: var(--kit-icon-button-disabled-opacity);
		cursor: not-allowed;
	}
	.dot {
		position: absolute;
		top: 18%;
		right: 18%;
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background: var(--kit-icon-button-dot-mark);
	}
</style>
