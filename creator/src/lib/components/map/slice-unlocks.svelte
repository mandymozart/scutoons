<script lang="ts">
	import { mapName, placesUnlockedBy, sliceUnlockPoint, type ChapterSliceRef, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { mapOf, setSliceUnlock } from '$lib/editor/map.ts';
	import { Icon } from '@scutoons/ui';

	/**
	 * The chapter editor's side of a place's unlock (owner 2026-10-01, bidirectional): "this slice unlocks: [Garten Eden]".
	 * It edits the SAME data as the map builder – the place's `unlock` in the world map (`{ toon, chapter, slice: <the
	 * slice's image>, at: <its reading point> }`); the map doc is saved / published with the chapter (like the cast).
	 */
	let {
		mapDoc,
		toon,
		chapter,
		image,
		slices,
		ontouch
	}: {
		mapDoc: ContentDoc;
		toon: string;
		chapter: number;
		/** the slice's image (its address) */
		image: string;
		/** the chapter's slices in reading order (scenes inlined) – for the reading point */
		slices: ChapterSliceRef[];
		/** the map was edited here (the chapter page saves it with the chapter) */
		ontouch: () => void;
	} = $props();

	const locale = $derived(getLocale());
	const map = $derived(mapOf(mapDoc) as WorldMap | null);
	const unlocked = $derived(placesUnlockedBy(map, toon, chapter, image));
	const name = (p: { id: string; name: Record<string, string> }) => mapName(p.name, locale) || p.id;
	const options = $derived.by(() => {
		const all = [...(map?.locations ?? [])].filter((p) => !unlocked.includes(p)).sort((a, b) => name(a).localeCompare(name(b)));
		return { own: all.filter((p) => p.toons?.includes(toon)), other: all.filter((p) => !p.toons?.includes(toon)) };
	});
	const point = $derived(sliceUnlockPoint(slices, image));

	function add(id: string) {
		if (!id || point === null) return;
		setSliceUnlock(mapDoc, id, { toon, chapter, slice: image, at: point });
		ontouch();
	}
	function remove(id: string) {
		setSliceUnlock(mapDoc, id, null);
		ontouch();
	}
</script>

<section class="unlocks" data-testid="slice-unlocks">
	<h4>{m['creator.slice_unlocks']()}</h4>
	<p class="hint">{m['creator.slice_unlocks_hint']()}</p>
	{#if mapDoc.loading}
		<p class="hint">{m['creator.loading']()}</p>
	{:else if !image}
		<p class="hint">{m['creator.slice_unlocks_no_image']()}</p>
	{:else}
		{#if unlocked.length}
			<ul>
				{#each unlocked as p (p.id)}
					<li data-testid="slice-unlock-{p.id}">
						<span class="name">{name(p)}</span>
						<a href="/map?location={encodeURIComponent(p.id)}" target="_blank" rel="noopener">{m['creator.slice_unlocks_on_map']()}</a>
						<button type="button" class="ghost" aria-label={m['creator.slice_unlocks_remove']()} title={m['creator.slice_unlocks_remove']()} onclick={() => remove(p.id)}><Icon name="x" /></button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="hint">{m['creator.slice_unlocks_none']()}</p>
		{/if}
		<select value="" onchange={(e) => { add(e.currentTarget.value); e.currentTarget.value = ''; }} aria-label={m['creator.slice_unlocks_add']()} data-testid="slice-unlock-add">
			<option value="">{m['creator.slice_unlocks_add']()}</option>
			<optgroup label={m['creator.slice_unlocks_this_toon']()}>
				{#each options.own as p (p.id)}
					<option value={p.id}>{name(p)}{p.unlock ? ` (${m['creator.slice_unlocks_elsewhere']({ chapter: p.unlock.chapter })})` : ''}</option>
				{/each}
			</optgroup>
			{#if options.other.length}
				<optgroup label={m['creator.slice_unlocks_other']()}>
					{#each options.other as p (p.id)}<option value={p.id}>{name(p)}</option>{/each}
				</optgroup>
			{/if}
		</select>
	{/if}
</section>

<style>
	.unlocks {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	h4 {
		margin: 0;
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	li {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	li .name {
		flex: 1;
		min-width: 0;
	}
</style>
