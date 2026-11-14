<script lang="ts">
	// Where a linked reader starts (#529, agents/research/SCENE-EVENTS.md §6.4 Q2): a toon, one of its chapters and a label
	// in it (optional – the chapter's start), "just like" the creator picks a jump's place. The places are the reader's
	// own: the toons of the menu (the root layout's), a toon's listed chapters (the library, `toonSeasons` – the toon
	// page's list) and a chapter's jump places (@scutoons/content `jumpLabels`, the creator's jump targets' list).
	import { jumpLabels, toonName, type ToonIndex } from '@scutoons/content';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { fetchChapter, fetchToon } from '$lib/api.ts';
	import { chapterName, toonSeasons, type ToonSeason } from '$lib/utils/chapter-name.ts';
	import type { ReaderPlace } from '$lib/conductor-reader.ts';

	let { place = $bindable() }: { place: ReaderPlace } = $props();

	const toons = $derived((page.data.toons as ToonIndex[] | undefined) ?? []);
	/** the picked toon's seasons (null: none picked or not loaded) */
	let seasons = $state<ToonSeason[] | null>(null);
	/** the picked chapter's jump places (null: none picked or not loaded) */
	let labels = $state<string[] | null>(null);
	let problem = $state('');
	const chapters = $derived(seasons?.flatMap((s) => s.chapters.map((c) => c.number)) ?? []);

	const failed = (e: unknown) => (problem = m['conductor_reader.failed']({ why: e instanceof Error ? e.message : String(e) }));

	async function pickToon(toon: string) {
		place = { toon, chapter: null, label: '' };
		seasons = labels = null;
		problem = '';
		if (!toon) return;
		try {
			const found = toonSeasons(await fetchToon(toon, getLocale()));
			if (place.toon === toon) seasons = found;
		} catch (e) {
			if (place.toon === toon) failed(e);
		}
	}

	async function pickChapter(value: string) {
		const toon = place.toon;
		const chapter = value === '' ? null : Number(value);
		place = { toon, chapter, label: '' };
		labels = null;
		problem = '';
		if (chapter === null) return;
		const still = () => place.toon === toon && place.chapter === chapter;
		try {
			const found = jumpLabels((await fetchChapter(toon, chapter, getLocale())).groups);
			if (still()) labels = found;
		} catch (e) {
			if (still()) (labels = []), failed(e);
		}
	}
</script>

<div class="pick">
	<label>
		{m['conductor_reader.toon']()}
		<select value={place.toon} onchange={(e) => pickToon(e.currentTarget.value)} data-testid="conductor-reader-toon">
			<option value="">{m['conductor_reader.pick']()}</option>
			{#each toons as t (t.id)}<option value={t.id}>{toonName(t, getLocale())}</option>{/each}
		</select>
	</label>
	<label>
		{m['conductor_reader.chapter']()}
		<select value={place.chapter === null ? '' : String(place.chapter)} disabled={!seasons} onchange={(e) => pickChapter(e.currentTarget.value)} data-testid="conductor-reader-chapter">
			<option value="">{m['conductor_reader.pick']()}</option>
			{#each chapters as n (n)}<option value={String(n)}>{chapterName(n, seasons ?? [])}</option>{/each}
		</select>
	</label>
	<label>
		{m['conductor_reader.label']()}
		<select value={place.label} disabled={!labels} onchange={(e) => (place = { ...place, label: e.currentTarget.value })} data-testid="conductor-reader-label">
			<option value="">{m['conductor_reader.start']()}</option>
			{#each labels ?? [] as l (l)}<option value={l}>{l}</option>{/each}
		</select>
	</label>
</div>
{#if problem}<p class="note" role="alert">{problem}</p>{/if}

<style>
	.pick {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: var(--space-1);
	}
	label {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	select {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		padding: 0 var(--space-05);
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		font-size: var(--type-body-size);
		text-transform: none;
		letter-spacing: normal;
	}
	select:focus {
		border-color: var(--ink);
	}
	.note {
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--badge-warning);
		font-size: var(--type-caption-size);
	}
</style>
