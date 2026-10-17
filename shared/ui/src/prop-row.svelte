<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One row of a compact inspector (agents/research/EDITOR-INPUTS.md): the tool label (caption, uppercase) in a left
	 * column, the controls sharing the rest in equal parts (a child with `data-prop-grow="2"` takes two parts). Below an
	 * 18rem wide row the label goes above the controls. The controls are a group named by the label; each control keeps
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
	.label {
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
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
	}
	.controls > :global(*) {
		flex: 1 1 0;
		min-width: 0;
	}
	.controls > :global([data-prop-grow='2']) {
		flex-grow: 2;
	}
	/* a segmented control that isn't stretched (units, 400 / 700) keeps its own width */
	.controls > :global(:is([data-prop-fit], .sc-seg:not(.is-stretch))) {
		flex: none;
	}
	@container (max-width: 18rem) {
		.inner {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
