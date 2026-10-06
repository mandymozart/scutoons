<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, JumpToTargetType, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * A jump's target (a jumpTo event, a choice's jump action): chapter + group, or a random group of a list (never the
	 * current one). Renders its fields into the parent's field grid.
	 */
	let { doc, path, target, groupLabels }: { doc: ContentDoc; path: Path; target: any; groupLabels: string[] } = $props();

	function toggleAnyOf(labelName: string, on: boolean) {
		doc.edit((file) => {
			const t = getAt(file, path);
			if (!t) return;
			const list: string[] = Array.isArray(t.anyOf) ? t.anyOf.filter((l: string) => l !== labelName) : [];
			if (on) list.push(labelName);
			if (list.length) t.anyOf = groupLabels.filter((l) => list.includes(l)).concat(list.filter((l) => !groupLabels.includes(l)));
			else delete t.anyOf;
		}, `${path.join('.')}.anyOf`);
	}
</script>

<PropField {doc} path={[...path, 'type']} label="target type" type="select" options={Object.values(JumpToTargetType)} required />
<PropField {doc} path={[...path, 'value']} label="value (chapter no.)" required />
{#if !target?.anyOf?.length}
	<PropField {doc} path={[...path, 'label']} label="label (group)" type={groupLabels.length ? 'select' : 'text'} options={groupLabels} placeholder={m['creator.vn_group_first']()} />
{/if}
{#if groupLabels.length}
	<fieldset class="any-of">
		<legend title={m['creator.vn_random_hint']()}>{m['creator.vn_random']()}</legend>
		{#each groupLabels as gl (gl)}
			<label><input type="checkbox" checked={target?.anyOf?.includes(gl) ?? false} onchange={(e) => toggleAnyOf(gl, e.currentTarget.checked)} /> {gl}</label>
		{/each}
	</fieldset>
{/if}

<style>
	.any-of {
		grid-column: 1 / -1;
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 0.75rem;
		margin: 0;
	}
	.any-of label {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}
</style>
