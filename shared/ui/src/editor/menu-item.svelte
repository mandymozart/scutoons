<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A line of a menu (deck-kit controls.rs `actions` / `checks`): a control high, its words a pad in, an icon before them
	 * where it helps; the hover ground under the pointer, none on one that cannot be done now (muted). `kbd` puts keys at
	 * its end, `external` the arrow out (it leaves for the browser), `sub` a chevron (it opens a panel beside), `on` a dot
	 * before it (a check list's line that is on; `on={false}` keeps the dot's room). `href` makes it a link; neither `href`
	 * nor `onclick` makes it a plain line of text. `current` / `expanded` keep the hover ground.
	 */
	type Props = {
		label?: string;
		icon?: string;
		href?: string;
		onclick?: (e: MouseEvent) => void;
		kbd?: string;
		external?: boolean;
		sub?: boolean;
		on?: boolean;
		current?: boolean;
		expanded?: boolean;
		disabled?: boolean;
		muted?: boolean;
		dim?: boolean;
		children?: Snippet;
	} & Omit<HTMLButtonAttributes, 'children' | 'onclick'> &
		Omit<HTMLAnchorAttributes, 'children' | 'onclick'>;

	let { label, icon, href, onclick, kbd, external = false, sub = false, on, current = false, expanded, disabled = false, muted = false, dim = false, role = 'menuitem', children, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if on !== undefined}<span class="tick" class:lit={on} aria-hidden="true"></span>{/if}
	{#if icon}<Icon name={icon} size="1rem" strokeWidth={2} />{/if}
	<span class="words">{#if children}{@render children()}{:else}{label}{/if}</span>
	{#if kbd}<kbd>{kbd}</kbd>{/if}
	{#if external}<Icon name="arrow-up-right" size="1rem" strokeWidth={2} />{:else if sub}<Icon name="chevron-right" size="1rem" strokeWidth={2} />{/if}
{/snippet}

{#if href && !disabled}
	<a {href} {role} aria-current={current ? 'page' : undefined} target={external ? '_blank' : undefined} rel={external ? 'noopener' : undefined} {onclick} {...rest as HTMLAnchorAttributes} class="kit-menu-item {rest.class ?? ''}" class:current class:muted class:dim>{@render inside()}</a>
{:else if onclick || disabled}
	<button type="button" {role} {disabled} aria-expanded={expanded} {onclick} {...rest as HTMLButtonAttributes} class="kit-menu-item {rest.class ?? ''}" class:current class:muted class:dim>{@render inside()}</button>
{:else}
	<span {...rest as Record<string, unknown>} class="kit-menu-item line {rest.class ?? ''}" class:muted class:dim>{@render inside()}</span>
{/if}

<style>
	.kit-menu-item {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: flex-start;
		gap: var(--kit-icon-gap);
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--editor-pad);
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-text);
		font: inherit;
		line-height: 1.2;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	.line {
		cursor: default;
	}
	.kit-menu-item :global(svg) {
		flex: none;
		width: var(--kit-icon);
		height: var(--kit-icon);
	}
	.words {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	kbd {
		flex: none;
		color: var(--tool-muted);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
	}
	.muted,
	.kit-menu-item:disabled {
		color: var(--kit-menu-item-disabled-text);
		cursor: default;
	}
	.dim {
		opacity: 0.55;
	}
	.current,
	.kit-menu-item[aria-expanded='true'],
	.kit-menu-item[data-kit-state~='hover'] {
		background: var(--kit-menu-item-hover-ground);
	}
	@media (hover: hover) {
		a.kit-menu-item:hover,
		button.kit-menu-item:hover:not(:disabled) {
			background: var(--kit-menu-item-hover-ground);
		}
	}
	.kit-menu-item:focus-visible,
	.kit-menu-item[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	/* a check list's dot (controls.rs `checks`: the text colour, 0.21rem round) and its room when off */
	.tick {
		flex: none;
		width: 0.42rem;
		height: 0.42rem;
		margin: 0 calc(var(--editor-pad) * 0.5);
		border-radius: 50%;
	}
	.tick.lit {
		background: var(--kit-menu-item-on-mark);
	}
</style>
