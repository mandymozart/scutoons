<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	/**
	 * A pill (deck-kit bar.rs `pill`): the floating panel's see-through ground, fully round, its line; text and icons carry
	 * the halo. What sits inside – IconButton, Button, PillTab – is inset by --kit-inset, the pill's height less twice the
	 * inset, and rounded by the pill's radius less the inset (their corners share a centre: deck-kit's `pills_rounded`),
	 * `--kit-seam` apart. `as` picks the element (a bar's `nav`, a status `footer`); `grow` lets it take the room left; `status` is the lower
	 * pill of the status bar; `vertical` stands it up as a column (a tool strip beside a canvas), the same pill turned. A group of icons and a toolbar always keep the pill's ground and line (owner 2026-11-24); a
	 * single icon stands alone as a round IconButton.
	 */
	type Props = {
		label?: string;
		as?: 'div' | 'span' | 'nav' | 'footer' | 'header';
		grow?: boolean;
		status?: boolean;
		vertical?: boolean;
		children: Snippet;
	} & Omit<HTMLAttributes<HTMLElement>, 'children'>;

	let { label, as = 'div', grow = false, status = false, vertical = false, children, ...rest }: Props = $props();
</script>

<svelte:element this={as} role={label && as !== 'nav' ? 'group' : undefined} aria-label={label} aria-orientation={vertical ? 'vertical' : undefined} {...rest} class="kit-pill {rest.class ?? ''}" class:grow class:status class:vertical>{@render children()}</svelte:element>

<style>
	.kit-pill {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: var(--kit-seam);
		box-sizing: border-box;
		min-width: 0;
		--pill: var(--kit-bar);
		height: var(--pill);
		/* the inside starts --kit-inset from the outer edge, the line included */
		padding: 0 calc(var(--kit-inset) - var(--editor-edge));
		border: var(--editor-edge) solid var(--kit-pill-rest-border);
		border-radius: calc(var(--pill) / 2);
		background: var(--kit-pill-rest-ground);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
		font-family: var(--editor-sans);
		--kit-h: calc(var(--pill) - 2 * var(--kit-inset));
		--kit-r: calc(var(--pill) / 2 - var(--kit-inset));
		--kit-icon-in: var(--kit-icon-alone);
	}
	.kit-pill :global(svg) {
		filter: var(--tool-icon-halo);
	}
	/* the status bar's pill: lower (deck-kit status.rs `height`) */
	.status {
		--pill: var(--kit-status);
		--kit-font: var(--editor-small);
		font-size: var(--editor-small);
	}
	.grow {
		flex: 1 1 auto;
	}
	/* a column: the pill's height is its width, the inset runs top and bottom */
	.vertical {
		flex-direction: column;
		width: var(--pill);
		height: auto;
		padding: calc(var(--kit-inset) - var(--editor-edge)) 0;
	}
</style>
