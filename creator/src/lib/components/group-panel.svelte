<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { Renderer } from '@scutoons/content';
	import PropField from './prop-field.svelte';

	/** Group settings: label, renderer type + params. For ref groups the doc is the referenced scene. */
	let { doc, groupPath, refSrc = null, toon = '' }: { doc: ContentDoc; groupPath: Path; refSrc?: string | null; toon?: string } = $props();

	const group = $derived(doc.get(doc.primary, groupPath));
	const params = $derived(group?.renderer?.params ?? {});
	const p = (...keys: (string | number)[]) => [...groupPath, ...keys];
</script>

<section class="group-panel" data-testid="group-panel">
	<!-- the label is the name; the scene's storage key is an internal ID (owner 2026-10-03) -->
	<h3>Group {group?.label ?? ''}</h3>
	<div class="grid g3">
		<PropField {doc} path={p('label')} label="label (?label=, jumpTo)" />
		<PropField {doc} path={p('renderer', 'type')} label="renderer" type="select" options={Object.values(Renderer)} required />
		{#if group?.interaction !== undefined}
			<PropField {doc} path={p('interaction')} label="interaction (untyped, kept)" />
		{/if}
	</div>
	<!-- the swiper's settings only for a horizontal group (owner 2026-10-03) -->
	{#if group?.renderer?.type === Renderer.HORIZONTAL}
	<details open={Object.keys(params).length > 0}>
		<summary>renderer params (swiper)</summary>
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
	{/if}
</section>

<style>
	.group-panel {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
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
