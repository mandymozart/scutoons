<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One row of a compact inspector (agents/research/EDITOR-INPUTS.md): the tool label (caption, uppercase) in a left
	 * column, the controls sharing the rest in equal parts (a child with `data-prop-grow="2"` takes two parts; one with
	 * `data-prop-line` starts its own line under the others, the label stays with the first). Below a
	 * 22rem wide row the label goes above the controls. The controls are a group named by the label; each control keeps
	 * its own accessible name (X, Y …).
	 */
	let {
		label,
		hint,
		testid,
		children
	}: {
		label: string;
		/** the label's tooltip (an explanation) */
		hint?: string;
		testid?: string;
		children: Snippet;
	} = $props();

	const uid = $props.id();
</script>

<div class="sc-prop-row" data-testid={testid}>
	<div class="inner">
		<span class="label" id="prop-{uid}" title={hint}>{label}</span>
		<div class="controls" role="group" aria-labelledby="prop-{uid}">{@render children()}</div>
	</div>
</div>

<style>
	.sc-prop-row {
		container-type: inline-size;
		min-width: 0;
	}
	.inner {
		display: grid;
		grid-template-columns: var(--prop-label-width, 5rem) minmax(0, 1fr);
		align-items: center;
		gap: var(--space-025) var(--space-05);
	}
	/* in line with the first line of controls (a row can have two) */
	.label {
		align-self: start;
		display: flex;
		align-items: center;
		min-height: var(--control-height);
		min-width: 0;
		overflow: hidden;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
	}
	.controls > :global(*) {
		flex: 1 1 0;
		min-width: 0;
	}
	/* a second line in the same row (the Size row: the sizing segments, then W ⟷ H under them) */
	.controls > :global([data-prop-line]) {
		flex-basis: 100%;
	}
	.controls > :global([data-prop-grow='2']) {
		flex-grow: 2;
	}
	/* a segmented control that isn't stretched (units, 400 / 700) keeps its own width */
	.controls > :global(:is([data-prop-fit], .sc-seg:not(.is-stretch))) {
		flex: none;
	}
	/* narrower than 22rem (the default 16rem dock panel): the label above, the controls the whole width – a number with
	   its unit dropdown needs the room (owner 2026-10-18, units per field) */
	@container (max-width: 22rem) {
		.inner {
			grid-template-columns: minmax(0, 1fr);
		}
		.label {
			min-height: 0;
		}
	}
</style>
