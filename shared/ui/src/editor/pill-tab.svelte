<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import CloseIcon from '../close-icon.svelte';
	import Icon from '../icon.svelte';

	/**
	 * One tab of a pill (deck-kit bar.rs `tabs`, controls.rs `pills_rounded`): its icon and words on one line, no border at
	 * rest (RULES H4): the one `on` in the text colour, the others muted; a rounded border shows on hover and while dragged
	 * (`data-kit-state="dragged"`); rounded by --kit-r (in a Pill: the pill's radius less the inset – concentric). The
	 * states' colours are kit.css's `--kit-tab-*`. `href` makes it a link (`aria-current` on), else a button (`aria-pressed`).
	 * `onclose` adds the close at its end (a square as high as the tab); `small` takes the small type (a pill of modes).
	 * `dim` mutes one that does not fit. Other attributes go to the tab itself (data-* for the caller).
	 */
	type Props = {
		label: string;
		on?: boolean;
		href?: string;
		onclick?: () => void;
		icon?: string;
		title?: string;
		dim?: boolean;
		disabled?: boolean;
		small?: boolean;
		closeLabel?: string;
		onclose?: () => void;
		trail?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'onclick' | 'title'>;

	let { label, on = false, href, onclick, icon, title, dim = false, disabled = false, small = false, closeLabel, onclose, trail, ...rest }: Props = $props();
</script>

{#snippet inside()}
	{#if icon}<Icon name={icon} size="1rem" strokeWidth={2} />{/if}<span class="words">{label}</span>{@render trail?.()}
{/snippet}

<div {...rest} class="kit-pill-tab {rest.class ?? ''}" class:on class:dim class:small class:closes={!!onclose} title={title ?? label}>
	{#if href}
		<a class="open" {href} aria-current={on ? 'page' : undefined} onclick={() => onclick?.()}>{@render inside()}</a>
	{:else}
		<button type="button" class="open" aria-pressed={on} {disabled} onclick={() => onclick?.()}>{@render inside()}</button>
	{/if}
	{#if onclose}
		<button type="button" class="close" aria-label={closeLabel ?? label} title={closeLabel ?? label} onclick={onclose}><CloseIcon size="0.8rem" stroke={1} /></button>
	{/if}
</div>

<style>
	.kit-pill-tab {
		display: inline-flex;
		flex: none;
		align-items: center;
		box-sizing: border-box;
		max-width: 14rem;
		height: var(--kit-h, var(--control-height));
		border-radius: var(--kit-r, var(--editor-radius));
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-rest-border);
		background: var(--kit-tab-rest-ground);
		color: var(--kit-tab-rest-text);
		font-size: var(--kit-font, var(--editor-font));
	}
	.small {
		font-size: var(--editor-small);
	}
	.dim {
		opacity: 0.55;
	}
	.on {
		color: var(--kit-tab-on-text);
	}
	.kit-pill-tab[data-kit-state~='hover'] {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-hover-border);
		color: var(--kit-tab-hover-text);
	}
	@media (hover: hover) {
		.kit-pill-tab:hover {
			box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-hover-border);
			color: var(--kit-tab-hover-text);
		}
	}
	.kit-pill-tab[data-kit-state~='dragged'],
	.kit-pill-tab[aria-grabbed='true'] {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-dragged-border);
		background: var(--kit-tab-dragged-ground);
		color: var(--kit-tab-dragged-text);
		text-shadow: none;
	}
	.kit-pill-tab:has(.open:disabled) {
		opacity: var(--kit-tab-disabled-opacity);
	}
	.open {
		display: inline-flex;
		flex: 1 1 auto;
		align-items: center;
		gap: var(--kit-icon-gap);
		box-sizing: border-box;
		min-width: 0;
		height: 100%;
		margin: 0;
		padding: 0 var(--editor-pad);
		border: 0;
		border-radius: inherit;
		background: none;
		color: inherit;
		font: inherit;
		line-height: 1;
		text-decoration: none;
		text-shadow: inherit;
		white-space: nowrap;
		cursor: pointer;
	}
	/* the close takes the tab's end: its words stop a pad short of it */
	.closes .open {
		padding-right: 0;
	}
	.open :global(svg) {
		flex: none;
		width: var(--kit-icon);
		height: var(--kit-icon);
	}
	.open:disabled {
		cursor: not-allowed;
	}
	.words {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.close {
		display: inline-grid;
		flex: none;
		place-items: center;
		box-sizing: border-box;
		width: var(--kit-h, var(--control-height));
		height: 100%;
		margin: 0;
		/* the hover ground sits 3px inside the square (bar.rs) */
		padding: calc(3 * var(--editor-edge));
		border: 0;
		border-radius: var(--kit-r, var(--editor-radius));
		background: none;
		background-clip: content-box;
		color: var(--tool-muted);
		cursor: pointer;
	}
	.on .close {
		color: var(--kit-tab-on-text);
	}
	@media (hover: hover) {
		.close:hover {
			background-color: var(--tool-hover);
			color: var(--tool-text);
		}
	}
	.open:focus-visible,
	.close:focus-visible,
	.kit-pill-tab[data-kit-state~='focus'] .open {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
</style>
