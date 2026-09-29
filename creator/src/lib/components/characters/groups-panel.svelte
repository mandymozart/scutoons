<script lang="ts">
	import { characterName, groupMembers, pickText, slugId, uniqueId, type CharacterRegistry, type GroupVocabulary, type LocaleText } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { edit, ensureGroups, groupsOf } from '$lib/editor/characters.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '../credits/locale-map-field.svelte';

	/** The shared groups (`scutoons/_/groups.json`): name / description / color per group and its members (affiliations). */
	let {
		groups,
		registry,
		languages,
		onselect
	}: {
		groups: ContentDoc;
		registry: CharacterRegistry | null;
		/** every toon language, the main source first */
		languages: string[];
		onselect: (characterId: string, persona?: string) => void;
	} = $props();

	const file = $derived(groupsOf(groups));
	let name = $state('');

	function add() {
		const text = name.trim();
		if (!text) return;
		ensureGroups(groups);
		const id = uniqueId(slugId(text), (groupsOf(groups)?.groups ?? []).map((g) => g.id));
		edit<GroupVocabulary>(groups, (f) => f.groups.push({ id, name: Object.fromEntries(languages.map((l, i) => [l, i === 0 ? text : ''])) }));
		name = '';
	}
	function change(i: number, fn: (g: GroupVocabulary['groups'][number]) => void, key?: string) {
		edit<GroupVocabulary>(groups, (f) => f.groups[i] && fn(f.groups[i]), key);
	}
	const clean = (text: LocaleText) => (Object.values(text).some((t) => t?.trim()) ? text : undefined);
</script>

<section class="groups" data-testid="groups-panel">
	<p class="hint">{m['creator.characters_groups_hint']()}</p>
	{#each file?.groups ?? [] as g, i (g.id)}
		{@const members = groupMembers(g.id, registry)}
		<article class="group" data-testid="group-{g.id}">
			<header>
				<span class="swatch" style:background={g.color || 'transparent'}></span>
				<strong>{pickText(g.name, languages[0], languages)}</strong>
				<code class="hint">{g.id}</code>
				<button
					type="button"
					class="ghost"
					onclick={() => {
						if (confirm(m['creator.characters_remove_group_confirm']({ name: pickText(g.name, languages[0], languages), count: members.length }))) edit<GroupVocabulary>(groups, (f) => f.groups.splice(i, 1));
					}}>✕</button
				>
			</header>
			<LocaleMapField value={g.name} {languages} label={m['creator.characters_name']()} required onchange={(v) => change(i, (x) => (x.name = v), `${g.id}:name`)} />
			<LocaleMapField value={g.description} {languages} label={m['creator.characters_description']()} multiline onchange={(v) => change(i, (x) => { const t = clean(v); if (t) x.description = t; else delete x.description; }, `${g.id}:description`)} />
			<label class="color">
				{m['creator.characters_color']()}
				<input type="text" value={g.color ?? ''} oninput={(e) => change(i, (x) => { const v = e.currentTarget.value.trim(); if (v) x.color = v; else delete x.color; }, `${g.id}:color`)} />
			</label>
			<div class="members" data-testid="group-members-{g.id}">
				<strong>{m['creator.characters_members']({ count: members.length })}</strong>
				{#each members as { character, persona, affiliation } (character.id + persona)}
					<button type="button" class="member" onclick={() => onselect(character.id, persona)}>
						{characterName(character, languages[0], persona, languages)}{#if pickText(affiliation.role, languages[0], languages)}<span class="hint"> · {pickText(affiliation.role, languages[0], languages)}</span>{/if}
					</button>
				{:else}
					<span class="hint">–</span>
				{/each}
			</div>
		</article>
	{/each}
	<span class="add">
		<input type="text" bind:value={name} placeholder={m['creator.characters_new_group']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), add())} data-testid="new-group" />
		<button type="button" class="secondary" onclick={add} disabled={!name.trim()}>+ {m['creator.characters_add_group']()}</button>
	</span>
</section>

<style>
	.groups {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.group {
		border: 2px solid var(--border-muted);
		padding: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.group header {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	.group header button {
		margin-left: auto;
	}
	.swatch {
		width: 1rem;
		height: 1rem;
		border: 1px solid var(--ink);
	}
	.color {
		font-size: 11px;
		font-weight: 700;
		max-width: 14rem;
	}
	.members {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		align-items: center;
		font-size: 12px;
	}
	.member {
		all: unset;
		cursor: pointer;
		border: 1px solid var(--border-subtle);
		padding: 0.1rem 0.4rem;
	}
	.member:hover {
		border-color: var(--ink);
	}
	.add {
		display: flex;
		gap: 0.3rem;
		max-width: 26rem;
	}
</style>
