<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, JumpToTargetType, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';
	import { getJumpPlaces } from '$lib/editor/jump-places.svelte.ts';
	import { addressBooks } from '$lib/editor/address-book.svelte.ts';
	import { childNames, TARGETS } from '$lib/editor/address-targets.ts';

	/**
	 * A jump's target (a jumpTo event, a choice's jump action, a group's end): a chapter – of this toon or another one
	 * (owner 2026-10-06, "at label") – + group, or a random group of a list (never the current one); a season, a toon's
	 * page, a product, XR. Renders its fields into the parent's field grid. A chapter target's chapter and group are ONE
	 * typeahead over the address book of the toon it leads into (#477); the other types keep their value field.
	 * `groupLabels`: the open chapter's (live).
	 */
	let { doc, path, target, groupLabels }: { doc: ContentDoc; path: Path; target: any; groupLabels: string[] } = $props();

	const places = getJumpPlaces();
	const isChapter = $derived(target?.type === 'chapter');
	/** the toon the chapter target leads into ('' = this toon) */
	const here = $derived(places?.toon ?? doc.toon);
	const otherToon = $derived(typeof target?.toon === 'string' && target.toon && target.toon !== here ? (target.toon as string) : '');
	const toon = $derived(otherToon || here);
	const value = $derived(target?.value === undefined || target?.value === null ? '' : String(target.value));
	/** the chosen chapter's group labels: the open chapter's live, another chapter's from its toon's address book */
	const labels = $derived.by<string[]>(() => {
		if (!isChapter || !value) return [];
		if (!places || (!otherToon && value === String(places.chapter))) return groupLabels;
		const book = addressBooks.of(toon).book;
		return book ? childNames(book, 'chapter', value, 'group') : [];
	});
	const otherToons = $derived((places?.toons ?? []).filter((t) => t.id !== here));

	function edit(change: (t: any) => void, key: string) {
		doc.edit((file) => {
			const t = getAt(file, path);
			if (t) change(t);
		}, `${path.join('.')}.${key}`);
	}

	/** another toon (or this one again): its chapter's groups are others – the group choice starts over */
	function setToon(id: string) {
		edit((t) => {
			if (id) t.toon = id;
			else delete t.toon;
			delete t.label;
			delete t.anyOf;
		}, 'toon');
	}

	/** a picked group replaces the random list; another chapter's random list goes */
	function picked(t: any, _entry: unknown, before: Record<string, string | undefined>) {
		if (t.label || String(t.value ?? '') !== before.value) delete t.anyOf;
	}

	function toggleAnyOf(labelName: string, on: boolean) {
		edit((t) => {
			const list: string[] = Array.isArray(t.anyOf) ? t.anyOf.filter((l: string) => l !== labelName) : [];
			if (on) list.push(labelName);
			if (list.length) t.anyOf = labels.filter((l) => list.includes(l)).concat(list.filter((l) => !labels.includes(l)));
			else delete t.anyOf;
		}, 'anyOf');
	}
	const id = $derived(`f-${path.join('-')}-toon`);
</script>

<PropField {doc} path={[...path, 'type']} label={m['creator.target_type']()} type="select" options={Object.values(JumpToTargetType).map((t) => ({ value: t, label: m[`creator.target_${t}`]() }))} required />
<!-- `toon` belongs to chapter targets (validation: an error elsewhere) – shown then too, so it can be cleared -->
{#if places && ((isChapter && otherToons.length) || otherToon)}
	<div class="field" title={m['creator.target_toon_hint']()}>
		<label for={id}>{m['creator.target_in_toon']()}</label>
		<select {id} value={otherToon} onchange={(e) => setToon(e.currentTarget.value)} data-testid="target-toon">
			<option value="">({m['creator.target_this_toon']()})</option>
			{#each otherToons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
			{#if otherToon && !otherToons.some((t) => t.id === otherToon)}<option value={otherToon}>{otherToon} (?)</option>{/if}
		</select>
	</div>
{/if}
{#if isChapter}
	<AddressField {doc} {path} {toon} kinds={TARGETS.jump.kinds} fields={TARGETS.jump.fields} label={m['creator.target_place']()} onpick={picked} testid="target-place" />
{:else}
	<PropField {doc} path={[...path, 'value']} label={m['creator.target_value']()} required />
	{#if !target?.anyOf?.length}
		<PropField {doc} path={[...path, 'label']} label={m['creator.target_group']()} placeholder={m['creator.vn_group_first']()} testid="target-label" />
	{/if}
{/if}
{#if labels.length}
	<fieldset class="any-of">
		<legend title={m['creator.vn_random_hint']()}>{m['creator.vn_random']()}</legend>
		{#each labels as gl (gl)}
			<label><input type="checkbox" checked={target?.anyOf?.includes(gl) ?? false} onchange={(e) => toggleAnyOf(gl, e.currentTarget.checked)} /> {gl}</label>
		{/each}
	</fieldset>
{/if}

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
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
