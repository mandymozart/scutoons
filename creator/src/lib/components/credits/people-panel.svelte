<script lang="ts">
	import { personUsage, replacePerson, type CreditPerson, type PeopleRegistry, type ToonCredits } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { creditsOf, edit, newPerson, peopleOf, searchPeople } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from './locale-map-field.svelte';

	/**
	 * The global people registry (scutoons/_/people.json): search, add, edit, merge duplicates. Merging rewrites this
	 * toon's credits and leaves the old record as `merged_into` so other toons' credits still resolve.
	 */
	let {
		people,
		credits,
		languages,
		selected = $bindable<string | null>(null)
	}: { people: ContentDoc; credits: ContentDoc; languages: string[]; selected?: string | null } = $props();

	let query = $state('');
	let mergeTarget = $state('');
	let newName = $state('');
	const registry = $derived(peopleOf(people));
	const all = $derived(registry?.people ?? []);
	const usage = $derived(personUsage(creditsOf(credits)));
	const shown = $derived(query.trim() ? searchPeople(all, query, 200) : [...all].filter((p) => !p.merged_into).sort((a, b) => (a.sort_name ?? a.name).localeCompare(b.sort_name ?? b.name)));
	const index = $derived(all.findIndex((p) => p.id === selected));
	const person = $derived(index >= 0 ? all[index] : null);

	const change = (fn: (p: CreditPerson) => void, key?: string) => {
		const i = index;
		edit<PeopleRegistry>(people, (file) => fn(file.people[i]), key ? `${selected}:${key}` : undefined);
	};
	const optional = (value: string) => (value.trim() ? value : undefined);
	function setField(field: 'name' | 'sort_name' | 'image', value: string) {
		change((p) => {
			const v = field === 'name' ? value : optional(value);
			if (v === undefined) delete p[field];
			else p[field] = v;
		}, field);
	}

	function add() {
		if (!registry || !newName.trim()) return;
		const p = newPerson(newName, registry);
		edit<PeopleRegistry>(people, (file) => file.people.push(p));
		selected = p.id;
		newName = '';
	}

	function merge() {
		const from = person;
		const to = all.find((p) => p.id === mergeTarget);
		if (!from || !to || from.id === to.id) return;
		const refs = usage.get(from.id) ?? 0;
		if (!confirm(m['creator.credits_merge_confirm']({ from: from.name, to: to.name, count: refs }))) return;
		edit<ToonCredits>(credits, (file) => void replacePerson(file, from.id, to.id));
		edit<PeopleRegistry>(people, (file) => {
			const target = file.people.find((p) => p.id === to.id)!;
			const source = file.people.find((p) => p.id === from.id)!;
			const aliases = new Set([...(target.aliases ?? []), ...(source.aliases ?? []), source.name].filter((n) => n !== target.name));
			if (aliases.size) target.aliases = [...aliases];
			for (const l of source.links ?? []) if (!(target.links ?? []).some((t) => t.url === l.url)) target.links = [...(target.links ?? []), l];
			if (!target.image && source.image) target.image = source.image;
			source.merged_into = to.id;
		});
		selected = to.id;
		mergeTarget = '';
	}
</script>

<div class="people" data-testid="people-panel">
	<div class="list">
		<input type="search" placeholder={m['creator.credits_search_people']()} bind:value={query} data-testid="people-search" />
		<ul>
			{#each shown as p (p.id)}
				<li>
					<button type="button" class="row" class:active={p.id === selected} onclick={() => (selected = p.id)} data-testid="people-row" data-person={p.id}>
						<span>{p.name}</span>
						<span class="hint">{usage.get(p.id) ?? 0}×</span>
					</button>
				</li>
			{:else}
				<li class="hint">{m['creator.credits_no_people']()}</li>
			{/each}
		</ul>
		<form
			class="new"
			onsubmit={(e) => {
				e.preventDefault();
				add();
			}}
		>
			<input type="text" placeholder={m['creator.credits_name']()} bind:value={newName} data-testid="new-person-name" />
			<button type="submit" class="secondary" disabled={!newName.trim()} data-testid="new-person">+ {m['creator.credits_add_person']()}</button>
		</form>
		<p class="hint">{m['creator.credits_people_hint']({ count: all.filter((p) => !p.merged_into).length })}</p>
	</div>

	{#if person}
		<div class="form" data-testid="person-form">
			<h3>{person.name} <span class="hint">{person.id}</span></h3>
			{#if person.merged_into}<p class="bad">{m['creator.credits_merged_into']({ id: person.merged_into })}</p>{/if}
			<label>{m['creator.credits_name']()}<input type="text" value={person.name} oninput={(e) => setField('name', e.currentTarget.value)} data-testid="person-name" /></label>
			<label>sort_name<input type="text" value={person.sort_name ?? ''} placeholder="Mozart, Mandy" oninput={(e) => setField('sort_name', e.currentTarget.value)} /></label>
			<label
				>{m['creator.credits_aliases']()}<input
					type="text"
					value={(person.aliases ?? []).join(', ')}
					oninput={(e) =>
						change((p) => {
							const list = e.currentTarget.value.split(',').map((s) => s.trim()).filter(Boolean);
							if (list.length) p.aliases = list;
							else delete p.aliases;
						}, 'aliases')}
				/></label
			>
			<label>{m['creator.credits_image']()}<input type="text" value={person.image ?? ''} placeholder="people/…jpg" oninput={(e) => setField('image', e.currentTarget.value)} /></label>
			<fieldset>
				<legend>{m['creator.credits_links']()}</legend>
				{#each person.links ?? [] as link, j (j)}
					<div class="link">
						<input type="text" value={link.label} placeholder="Instagram" oninput={(e) => change((p) => (p.links![j].label = e.currentTarget.value), `link-label-${j}`)} />
						<input type="url" value={link.url} placeholder="https://…" oninput={(e) => change((p) => (p.links![j].url = e.currentTarget.value), `link-url-${j}`)} />
						<button type="button" class="ghost" aria-label={m['creator.credits_remove']()} onclick={() => change((p) => (p.links!.splice(j, 1), p.links!.length || delete p.links))}>✕</button>
					</div>
				{/each}
				<button type="button" class="ghost" onclick={() => change((p) => (p.links = [...(p.links ?? []), { label: '', url: '' }]))}>+ {m['creator.credits_link']()}</button>
			</fieldset>
			<LocaleMapField
				value={person.bio}
				{languages}
				multiline
				label="bio"
				onchange={(v) =>
					change((p) => {
						if (Object.values(v).some((t) => t.trim())) p.bio = v;
						else delete p.bio;
					}, 'bio')}
			/>
			<div class="merge">
				<select bind:value={mergeTarget} aria-label={m['creator.credits_merge_into']()} data-testid="merge-target">
					<option value="">{m['creator.credits_merge_into']()} …</option>
					{#each all.filter((p) => p.id !== person.id && !p.merged_into) as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
				</select>
				<button type="button" class="secondary" disabled={!mergeTarget} onclick={merge} data-testid="merge">{m['creator.credits_merge']()}</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.people {
		display: grid;
		grid-template-columns: minmax(14rem, 1fr) minmax(0, 2fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 800px) {
		.people {
			grid-template-columns: 1fr;
		}
	}
	.list {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 28rem;
		overflow: auto;
		border: 1px solid var(--border-subtle);
	}
	.row {
		all: unset;
		box-sizing: border-box;
		display: flex;
		justify-content: space-between;
		width: 100%;
		padding: 0.25rem 0.5rem;
		cursor: pointer;
		font-size: 13px;
	}
	.row:hover,
	.row.active {
		background: var(--surface-hover);
	}
	.row.active {
		font-weight: 700;
	}
	.new {
		display: flex;
		gap: 0.25rem;
	}
	.new input {
		flex: 1;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.form h3 {
		margin: 0;
		font-size: 15px;
	}
	.form label {
		display: flex;
		flex-direction: column;
		font-size: 11px;
		gap: 0.1rem;
	}
	fieldset {
		border: 1px solid var(--border-subtle);
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	legend {
		font-size: 11px;
		font-weight: 700;
	}
	.link {
		display: grid;
		grid-template-columns: 8rem 1fr auto;
		gap: 0.25rem;
	}
	.merge {
		display: flex;
		gap: 0.25rem;
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.5rem;
	}
	.bad {
		color: var(--error);
	}
</style>
