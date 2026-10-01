<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { Renderer } from '@scutoons/content';
	import PropField from './prop-field.svelte';
	import { sceneLabel } from '$lib/labels.ts';

	/** Group settings: label, renderer type + params. For ref groups the doc is the referenced scene. */
	let { doc, groupPath, refSrc = null, toon = '' }: { doc: ContentDoc; groupPath: Path; refSrc?: string | null; toon?: string } = $props();

	const group = $derived(doc.get(doc.primary, groupPath));
	const params = $derived(group?.renderer?.params ?? {});
	const p = (...keys: (string | number)[]) => [...groupPath, ...keys];
</script>

<section class="group-panel" data-testid="group-panel">
	<h3>Group {group?.label ?? ''}{#if refSrc}<span class="ref"> · {sceneLabel(toon, refSrc)}</span>{/if}</h3>
	<div class="grid g3">
		<PropField {doc} path={p('label')} label="label (?label=, jumpTo)" />
		<PropField {doc} path={p('renderer', 'type')} label="renderer" type="select" options={Object.values(Renderer)} required />
		{#if group?.interaction !== undefined}
			<PropField {doc} path={p('interaction')} label="interaction (untyped, kept)" />
		{/if}
	</div>
	<details open={Object.keys(params).length > 0}>
		<summary>renderer params {group?.renderer?.type === Renderer.HORIZONTAL ? '(swiper)' : '(horizontal only)'}</summary>
		<div class="grid g4">
			<PropField {doc} path={p('renderer', 'params', 'autoPlay', 'enabled')} label="autoPlay" type="bool" placeholder="off" />
			<PropField {doc} path={p('renderer', 'params', 'autoPlay', 'delay')} label="autoPlay delay ms" type="number" step={100} placeholder="3000" />
			<PropField {doc} path={p('renderer', 'params', 'loop')} label="loop" type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'flip')} label="flip" type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'cube')} label="cube" type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'keyboard')} label="keyboard" type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'freeMode')} label="freeMode" type="bool" placeholder="false" />
		</div>
	</details>
</section>

<style>
	.group-panel {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.ref {
		color: var(--muted);
		font-weight: 400;
	}
	.grid {
		display: grid;
		gap: 0.4rem 0.6rem;
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.g4 {
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}
	summary {
		cursor: pointer;
		font-size: var(--type-caption-size);
		margin-bottom: 0.3rem;
	}
</style>
