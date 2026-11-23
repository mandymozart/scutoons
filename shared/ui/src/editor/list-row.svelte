<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A line of a list (deck-kit editor home.rs `lines`, controls.rs `tree`): a control high (`dense`: a tree row), its
	 * kind's icon muted half a pad in, its name, `detail` small and muted after it, `meta` at its end; the hover ground under
	 * the pointer, the pressed ground when `selected`. `lead` replaces the icon (an avatar), `children` goes at the end.
	 * `href` makes it a link, `onclick` a button, neither a plain line.
	 */
	type Props = {
		label: string;
		icon?: string;
		detail?: string;
		meta?: string;
		href?: string;
		onclick?: () => void;
		selected?: boolean;
		dense?: boolean;
		lead?: Snippet;
		children?: Snippet;
	} & Omit<HTMLAnchorAttributes, 'children' | 'onclick'>;

	let { label, icon, detail, meta, href, onclick, selected = false, dense = false, lead, children, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if lead}{@render lead()}{:else if icon}<Icon name={icon} size="1rem" strokeWidth={2} />{/if}
	<span class="name">{label}</span>
	{#if detail}<span class="detail">{detail}</span>{/if}
	{#if meta}<span class="meta">{meta}</span>{/if}
	{@render children?.()}
{/snippet}

{#if href}
	<a {href} {...rest} class="kit-list-row {rest.class ?? ''}" class:selected class:dense aria-current={selected ? 'true' : undefined}>{@render inside()}</a>
{:else if onclick}
	<button type="button" {onclick} {...rest as Record<string, unknown>} class="kit-list-row {rest.class ?? ''}" class:selected class:dense aria-pressed={selected}>{@render inside()}</button>
{:else}
	<div {...rest as Record<string, unknown>} class="kit-list-row {rest.class ?? ''}" class:selected class:dense>{@render inside()}</div>
{/if}

<style>
	.kit-list-row {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--kit-icon-gap);
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		border: 0;
		border-radius: var(--editor-radius);
		background: var(--kit-list-row-rest-ground);
		color: var(--kit-list-row-rest-text);
		font: inherit;
		text-align: left;
		text-decoration: none;
	}
	button.kit-list-row,
	a.kit-list-row {
		cursor: pointer;
	}
	.dense {
		min-height: var(--editor-tree-row);
	}
	.kit-list-row > :global(svg) {
		flex: none;
		width: var(--kit-icon);
		height: var(--kit-icon);
		color: var(--tool-muted);
	}
	.name {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.detail,
	.meta {
		flex: none;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		white-space: nowrap;
	}
	.meta {
		margin-left: auto;
	}
	.kit-list-row[data-kit-state~='hover'] {
		background: var(--kit-list-row-hover-ground);
	}
	@media (hover: hover) {
		a.kit-list-row:hover:not(.selected),
		button.kit-list-row:hover:not(.selected) {
			background: var(--kit-list-row-hover-ground);
		}
	}
	/* selected: the pressed fill, no side bar (RULES H10) */
	.selected {
		background: var(--kit-list-row-selected-ground);
		color: var(--kit-list-row-selected-text);
		text-shadow: none;
	}
	.selected > :global(svg),
	.selected .detail,
	.selected .meta {
		color: var(--kit-list-row-selected-text);
	}
	.kit-list-row[data-kit-state~='dragged'],
	.kit-list-row[aria-grabbed='true'] {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-list-row-dragged-border);
		background: var(--kit-list-row-dragged-ground);
		text-shadow: none;
	}
	.kit-list-row[aria-disabled='true'] {
		color: var(--kit-list-row-disabled-text);
		opacity: var(--kit-list-row-disabled-opacity);
	}
	.kit-list-row:focus-visible,
	.kit-list-row[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
</style>
