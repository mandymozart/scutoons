<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import AddressField from '$lib/components/fields/address-field.svelte';
	import { getJumpPlaces } from '$lib/editor/jump-places.svelte.ts';
	import { addressBooks } from '$lib/editor/address-book.svelte.ts';
	import { childNames, scopeOf, TARGETS } from '$lib/editor/address-targets.ts';

	/**
	 * A jump's target (a jumpTo event, a choice's jump action, a group's end) is ONE typeahead (#477 slice D; owner
	 * 2026-11-10 [typeahead]: "Should all this become a single field with type head ? Because the scope is in the typehead
	 * anyhow"): the entry picked is the target – a chapter or a group of this toon, a season, a product, the toon's page; a
	 * toon narrows the field to that toon's chapters and groups (a chip). The event keeps its fields (`type`, `value`,
	 * `label`, `toon`), filled from the entry and its scope. "Random, one of" stays its own control: a list, not a target.
	 * Renders its fields into the parent's field grid. `groupLabels`: the open chapter's (live).
	 */
	let { doc, path, target, groupLabels }: { doc: ContentDoc; path: Path; target: any; groupLabels: string[] } = $props();

	const places = getJumpPlaces();
	const here = $derived(places?.toon ?? doc.toon);
	/** the toon a chapter target leads into (another one's, else this one) */
	const toon = $derived(scopeOf(target, TARGETS.jump, here) ?? here);
	const value = $derived(target?.value === undefined || target?.value === null ? '' : String(target.value));
	/** the chosen chapter's group labels: the open chapter's live, another chapter's from its toon's address book */
	const labels = $derived.by<string[]>(() => {
		if (target?.type !== 'chapter' || !value) return [];
		if (!places || (toon === here && value === String(places.chapter))) return groupLabels;
		const book = addressBooks.of(toon).book;
		return book ? childNames(book, 'chapter', value, 'group') : [];
	});

	/** a picked group replaces the random list; another chapter, toon or kind of target drops it */
	function picked(t: any, _entry: unknown, before: Record<string, string | undefined>) {
		if (t.label || ['type', 'value', 'toon'].some((f) => String(t[f] ?? '') !== (before[f] ?? ''))) delete t.anyOf;
	}

	function toggleAnyOf(labelName: string, on: boolean) {
		doc.edit((file) => {
			const t = getAt(file, path);
			if (!t) return;
			const list: string[] = Array.isArray(t.anyOf) ? t.anyOf.filter((l: string) => l !== labelName) : [];
			if (on) list.push(labelName);
			if (list.length) t.anyOf = labels.filter((l) => list.includes(l)).concat(list.filter((l) => !labels.includes(l)));
			else delete t.anyOf;
		}, `${path.join('.')}.anyOf`);
	}
</script>

<div class="target" title={m['creator.target_toon_hint']()}>
	<AddressField {doc} {path} toon={here} spec={TARGETS.jump} label={m['creator.target_type']()} onpick={picked} testid="target-place" />
</div>
{#if labels.length}
	<fieldset class="any-of">
		<legend title={m['creator.vn_random_hint']()}>{m['creator.vn_random']()}</legend>
		{#each labels as gl (gl)}
			<label><input type="checkbox" checked={target?.anyOf?.includes(gl) ?? false} onchange={(e) => toggleAnyOf(gl, e.currentTarget.checked)} /> {gl}</label>
		{/each}
	</fieldset>
{/if}

<style>
	/* the one field takes the room of two of the parent's cells (it had three controls there) */
	.target {
		grid-column: span 2;
		min-width: 0;
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
