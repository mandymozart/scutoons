<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { DEFAULT_MOOD, MOOD_ID, sortedMoods, type CharacterRegistry, type LocaleText, type MoodVocabulary } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { edit, ensureMoods, moodsOf } from '$lib/editor/characters.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * The mood vocabulary (entity `moods`): open-ended – add, rename (label per language), reorder, mark as part of
	 * the standard set (a checklist, never required). Only `neutral` is required (the fallback anchor).
	 */
	let { moods, registry, languages }: { moods: ContentDoc; registry: CharacterRegistry | null; languages: string[] } = $props();

	const list = $derived(sortedMoods(moodsOf(moods)));
	let name = $state('');

	/** how many looks have a portrait for each mood */
	const usage = $derived.by(() => {
		const count = new Map<string, number>();
		for (const c of registry?.characters ?? [])
			for (const p of c.personas ?? [])
				for (const l of p.looks ?? [])
					for (const [mood, slots] of Object.entries(l.images ?? {})) if (slots?.portrait) count.set(mood, (count.get(mood) ?? 0) + 1);
		return count;
	});

	function change(fn: (v: MoodVocabulary) => void, key?: string) {
		ensureMoods(moods);
		edit<MoodVocabulary>(moods, fn, key);
	}
	/** renumbers `order` 1…n in the shown order, then applies `fn` to the sorted list */
	function reorder(fn: (sorted: MoodVocabulary['moods']) => void) {
		change((v) => {
			const sorted = sortedMoods(v);
			fn(sorted);
			sorted.forEach((x, i) => (x.order = i + 1));
			v.moods = sorted;
		});
	}
	function move(from: number, to: number) {
		if (to < 0 || to >= list.length) return;
		reorder((sorted) => {
			const [item] = sorted.splice(from, 1);
			sorted.splice(to, 0, item);
		});
	}
	function add() {
		const id = name.trim().toLowerCase().replace(/\s+/g, '-');
		if (!MOOD_ID.test(id) || list.some((x) => x.id === id)) return;
		const label: LocaleText = Object.fromEntries(languages.map((l, i) => [l, i === 0 ? name.trim() : '']));
		reorder((sorted) => sorted.push({ id, label }));
		name = '';
	}
</script>

<section class="moods" data-testid="moods-panel">
	<ol>
		{#each list as mood, i (mood.id)}
			<li data-testid="mood-{mood.id}">
				<span class="order">
					<button type="button" class="ghost" onclick={() => move(i, i - 1)} disabled={i === 0} aria-label={m['creator.credits_up']()}><Icon name="arrow-up" /></button>
					<button type="button" class="ghost" onclick={() => move(i, i + 1)} disabled={i === list.length - 1} aria-label={m['creator.credits_down']()}><Icon name="arrow-down" /></button>
				</span>
				<code>{mood.id}</code>
				<LocaleMapField value={mood.label} {languages} label={m['creator.characters_look_label']()} required onchange={(v) => change((x) => { const t = x.moods.find((y) => y.id === mood.id); if (t) t.label = v; }, `mood-${mood.id}`)} />
				<label class="check">
					<input type="checkbox" checked={!!mood.standard} onchange={(e) => change((x) => { const t = x.moods.find((y) => y.id === mood.id); if (!t) return; if (e.currentTarget.checked) t.standard = true; else delete t.standard; })} />
					{m['creator.characters_standard']()}
				</label>
				<span class="hint">{m['creator.characters_mood_used']({ count: usage.get(mood.id) ?? 0 })}</span>
				<button
					type="button"
					class="ghost"
					disabled={mood.id === DEFAULT_MOOD}
					title={mood.id === DEFAULT_MOOD ? m['creator.characters_neutral_required']() : ''}
					onclick={() => {
						if (confirm(m['creator.characters_remove_mood_confirm']({ mood: mood.id, count: usage.get(mood.id) ?? 0 }))) change((x) => (x.moods = x.moods.filter((y) => y.id !== mood.id)));
					}} aria-label={m['creator.remove']()}><CloseIcon /></button
				>
			</li>
		{/each}
	</ol>
	<span class="add">
		<input type="text" bind:value={name} placeholder={m['creator.characters_new_mood']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), add())} data-testid="moods-new" />
		<button type="button" class="secondary" onclick={add} disabled={!MOOD_ID.test(name.trim().toLowerCase().replace(/\s+/g, '-'))}><Icon name="plus" /> {m['creator.characters_add_mood']()}</button>
	</span>
</section>

<style>
	.moods {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		max-width: 60rem;
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	li {
		display: grid;
		grid-template-columns: auto 7rem minmax(0, 1fr) auto 6rem auto;
		gap: 0.5rem;
		align-items: end;
		border-bottom: 1px solid var(--border-subtle);
		padding-bottom: 0.3rem;
	}
	.order {
		display: flex;
	}
	.check {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.add {
		display: flex;
		gap: 0.3rem;
		max-width: 26rem;
	}
</style>
