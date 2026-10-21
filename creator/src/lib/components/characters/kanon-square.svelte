<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { KanonState } from '$lib/editor/kanon-docs.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A level's Kanon text square (research/CHARACTER-PAGE.md §3a K3, #82): at the end of the head, the persona row and the
	 * look row – a borderless icon square like the stage's view squares (hover = the outline, open = inverted) with a state
	 * dot: none = not written (hollow), draft, published. A click opens that level's Kanon in the inspector.
	 */
	let { state, open, label, onclick, testid }: { state: KanonState; open: boolean; label: string; onclick: () => void; testid: string } = $props();

	const STATE_LABELS: Record<KanonState, () => string> = {
		none: () => m['creator.scripts_not_written'](),
		draft: () => m['creator.status_draft'](),
		published: () => m['creator.status_published']()
	};
	const title = $derived(`${label} · ${STATE_LABELS[state]()}`);
	/** the dot's colour is the shared chip state (tools.css .sc-chip.is-*); not written = hollow */
	const CHIP: Record<KanonState, string> = { none: 'none', draft: 'sc-chip is-draft', published: 'sc-chip is-published' };
</script>

<button type="button" class="sq" aria-pressed={open} {title} aria-label={title} {onclick} data-state={state} data-testid={testid}>
	<Icon name="file-text" />
	<span class="dot {CHIP[state]}" aria-hidden="true"></span>
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
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sq[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	/* the state dot in the corner: a shared chip's colour (draft / published) drawn as a dot; hollow = not written */
	.dot {
		position: absolute;
		top: 0.3rem;
		right: 0.3rem;
		width: 0.45rem;
		height: 0.45rem;
		padding: 0;
		border-radius: 50%;
		box-sizing: border-box;
	}
	.dot.none {
		border: var(--border-width-regular) solid currentColor;
	}
</style>
