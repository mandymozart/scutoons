<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { KanonState } from '$lib/editor/kanon-docs.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A level's Kanon text square (research/CHARACTER-PAGE.md §3a K3): at the end of the head, the persona row and the
	 * look row – a borderless icon square like the stage's view squares (hover = the outline, open = inverted); a hollow
	 * dot while that level's Kanon is not written (a quiet mark of a missing value; publishing shows in the Publish pill).
	 * A click opens that level's Kanon in the inspector.
	 */
	let { state, open, label, onclick, testid }: { state: KanonState; open: boolean; label: string; onclick: () => void; testid: string } = $props();

	const title = $derived(state === 'none' ? `${label} · ${m['creator.scripts_not_written']()}` : label);
</script>

<button type="button" class="sq" aria-pressed={open} {title} aria-label={title} {onclick} data-state={state} data-testid={testid}>
	<Icon name="file-text" />
	{#if state === 'none'}<span class="dot" aria-hidden="true"></span>{/if}
</button>

<style>
	.sq {
		all: unset;
		position: relative;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		cursor: pointer;
	}
	.sq:hover,
	.sq:focus-visible {
		outline: var(--border-width-regular) solid var(--tool-text);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sq[aria-pressed='true'] {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	/* the hollow dot in the corner: not written */
	.dot {
		position: absolute;
		top: 0.3rem;
		right: 0.3rem;
		width: 0.45rem;
		height: 0.45rem;
		padding: 0;
		border-radius: 50%;
		box-sizing: border-box;
		border: var(--border-width-regular) solid currentColor;
	}
</style>
