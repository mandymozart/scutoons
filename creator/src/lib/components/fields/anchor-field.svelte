<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { ANCHORS, getAt, isAnchor, reanchor, type Anchor, type Path } from '@scutoons/content';
	import { drawnHeights, drawnKey } from '$lib/editor/drawn-heights.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The 9-point anchor of a text box / placed item (owner 2026-10-17, DESIGN.md Part 2 "Anchor"): a 3×3 radio group, the
	 * current point filled. Choosing another point never moves the box – x / y are converted (anchor.ts `reanchor`, with
	 * the canvas' drawn height when the rect has none). 'tl' is stored as no key (the default).
	 */
	let { doc, path }: { doc: ContentDoc; /** path of the entry (dialog / placed item) */ path: Path } = $props();

	const entry = $derived(doc.get(doc.primary, path));
	const current = $derived<Anchor>(isAnchor(entry?.anchor) ? entry.anchor : 'tl');
	const name = $derived(`anchor-${path.join('-')}`);

	function choose(next: Anchor) {
		if (next === current) return;
		const height = drawnHeights[drawnKey(path)];
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e?.rect) return;
			const { x, y } = reanchor(e.rect, e.anchor, next, height);
			e.rect.x = x;
			e.rect.y = y;
			if (next === 'tl') delete e.anchor;
			else e.anchor = next;
		}, `${path.join('.')}.anchor`);
	}
</script>

<fieldset class="anchor" title={m['creator.anchor_hint']()} data-testid="anchor-field">
	<legend>{m['creator.anchor']()}</legend>
	<div class="grid" role="radiogroup" aria-label={m['creator.anchor']()}>
		{#each ANCHORS as a (a)}
			<input
				type="radio"
				{name}
				value={a}
				checked={a === current}
				aria-label={m[`creator.anchor_${a}`]()}
				title={m[`creator.anchor_${a}`]()}
				data-testid="anchor-{a}"
				onchange={() => choose(a)}
			/>
		{/each}
	</div>
</fieldset>

<style>
	/* the legend is the label above it (tools.css); in the position row without the group's bottom margin */
	.anchor {
		flex: none;
		margin: 0;
		/* as wide as its label (tools.css gives fieldsets min-width 0) */
		min-width: max-content;
	}
	/* one square group at the control height: nine square cells in a joined frame (DESIGN "Round vs square") */
	.grid { /* grid-ok: a 3×3 picker of square cells, not a field grid */
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		width: var(--control-height);
		height: var(--control-height);
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
	}
	/* touch: bigger cells */
	@media (pointer: coarse) {
		.grid {
			width: calc(1.5 * var(--control-height));
			height: calc(1.5 * var(--control-height));
		}
	}
	input {
		appearance: none;
		margin: 0;
		width: 100%;
		height: 100%; /* control-ok: a cell of the one control-height group */
		min-width: 0;
		min-height: 0;
		border-radius: 0;
		background: var(--paper);
		cursor: pointer;
		display: grid;
		place-items: center;
	}
	/* a small dot per point; the chosen point filled ink */
	input::before {
		content: '';
		width: 30%;
		height: 30%; /* control-ok: the cell's dot, not a control */
		background: var(--text-muted);
	}
	input:hover::before {
		background: var(--ink);
	}
	input:checked {
		background: var(--ink);
	}
	input:checked::before {
		background: var(--paper);
	}
	input:focus-visible {
		outline: 2px solid var(--ink);
		outline-offset: 1px;
		position: relative;
		z-index: 1;
	}
</style>
