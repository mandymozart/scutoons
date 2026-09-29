<script lang="ts">
	import { clone, pickText, type Affiliation, type GroupVocabulary, type LocaleText, type StoryPoint } from '@scutoons/content';
	import type { ToonInfo } from '$lib/editor/characters.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '../credits/locale-map-field.svelte';

	/** A list of group memberships (character default or a persona's override): group, role per language, since. */
	let {
		value,
		groups,
		toons,
		languages,
		testid = 'affiliations',
		onchange
	}: {
		value: Affiliation[] | undefined;
		groups: GroupVocabulary | null;
		toons: ToonInfo[];
		languages: string[];
		testid?: string;
		onchange: (next: Affiliation[]) => void;
	} = $props();

	const list = $derived(value ?? []);
	const source = $derived(languages[0] ?? 'de');

	function update(i: number, fn: (a: Affiliation) => void) {
		const next = clone(list);
		fn(next[i]);
		onchange(next);
	}
	function point(p: StoryPoint | undefined, key: 'toon' | 'season' | 'chapter', raw: string): StoryPoint | undefined {
		const next: StoryPoint = { ...(p ?? { toon: '' }) };
		if (key === 'toon') next.toon = raw;
		else if (raw.trim() === '') delete next[key];
		else next[key] = Math.max(1, Math.round(Number(raw)) || 1);
		return next.toon ? next : undefined;
	}
	const clean = (t: LocaleText) => (Object.values(t).some((x) => x?.trim()) ? t : undefined);
</script>

<div class="affiliations" data-testid={testid}>
	{#each list as a, i (i)}
		<div class="row">
			<label>
				{m['creator.characters_group']()}
				<select value={a.group} onchange={(e) => update(i, (x) => (x.group = e.currentTarget.value))}>
					{#each groups?.groups ?? [] as g (g.id)}<option value={g.id}>{pickText(g.name, source, languages)}</option>{/each}
					{#if !groups?.groups.some((g) => g.id === a.group)}<option value={a.group}>{a.group} (?)</option>{/if}
				</select>
			</label>
			<LocaleMapField value={a.role} {languages} label={m['creator.characters_affiliation_role']()} onchange={(v) => update(i, (x) => { const t = clean(v); if (t) x.role = t; else delete x.role; })} />
			<label>
				{m['creator.characters_since']()}
				<select value={a.since?.toon ?? ''} onchange={(e) => update(i, (x) => { const p = point(x.since, 'toon', e.currentTarget.value); if (p) x.since = p; else delete x.since; })}>
					<option value="">–</option>
					{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
				</select>
			</label>
			{#each ['season', 'chapter'] as const as key (key)}
				<label>{key === 'season' ? m['creator.characters_season']() : m['creator.characters_chapter']()}<input type="number" min="1" disabled={!a.since} value={a.since?.[key] ?? ''} onchange={(e) => update(i, (x) => { const p = point(x.since, key, e.currentTarget.value); if (p) x.since = p; })} /></label>
			{/each}
			<button type="button" class="ghost" onclick={() => onchange(list.filter((_, j) => j !== i))} aria-label={m['creator.characters_remove']()}>✕</button>
		</div>
	{/each}
	{#if groups?.groups.length}
		<select
			value=""
			onchange={(e) => {
				const g = e.currentTarget.value;
				e.currentTarget.value = '';
				if (g) onchange([...clone(list), { group: g }]);
			}}
			data-testid="{testid}-add"
		>
			<option value="">+ {m['creator.characters_add_affiliation']()}</option>
			{#each groups.groups.filter((g) => !list.some((a) => a.group === g.id)) as g (g.id)}<option value={g.id}>{pickText(g.name, source, languages)}</option>{/each}
		</select>
	{:else}
		<p class="hint">{m['creator.characters_no_groups']()}</p>
	{/if}
</div>

<style>
	.affiliations {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-items: flex-start;
	}
	.row {
		display: grid;
		grid-template-columns: 10rem minmax(0, 1fr) 8rem 4rem 4rem auto;
		gap: 0.4rem;
		align-items: end;
		width: 100%;
	}
	label {
		font-size: 11px;
		font-weight: 700;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	@media (max-width: 900px) {
		.row {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
