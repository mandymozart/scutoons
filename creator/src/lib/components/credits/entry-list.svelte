<script lang="ts">
	import { CloseIcon, SortGrip, sortable } from '@scutoons/ui';
	import { getAt, Language, peopleIndex, roleIndex, roleText, type CreditEntry, type LocaleText, type Path, type RoleCategory, type ToonCredits } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { creditsOf, edit, move, newPerson, newRole, peopleOf, rolesOf } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PersonPicker from '$lib/components/credits/person-picker.svelte';
	import RolePicker from '$lib/components/credits/role-picker.svelte';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * An ordered list of `{ person, roles }` of a section (toon sections have two: lead + entries). Reorder with the
	 * shared Sortable (the ⋮⋮ grip: drag, or Alt+ArrowUp / ArrowDown); roles are chips (click = detail / language, × = remove).
	 */
	let {
		credits,
		people,
		roles,
		path,
		languages,
		testid,
		onperson
	}: {
		credits: ContentDoc;
		people: ContentDoc;
		roles: ContentDoc;
		/** path of the list inside the credits, e.g. ['sections', 0, 'lead'] */
		path: Path;
		languages: string[];
		testid: string;
		/** open a person in the People tab */
		onperson?: (id: string) => void;
	} = $props();

	const lang = $derived(languages[0] ?? 'en');
	const list = $derived((getAt(creditsOf(credits), path) ?? []) as CreditEntry[]);
	const peopleMap = $derived(peopleIndex(peopleOf(people)));
	const roleMap = $derived(roleIndex(rolesOf(roles), creditsOf(credits)));
	const roleList = $derived([...roleMap.values()]);
	const listed = $derived(new Set(list.map((e) => e.person)));
	let open = $state<string | null>(null); // `${entry}:${role}` whose details are shown

	const change = (fn: (list: CreditEntry[], file: ToonCredits) => void, key?: string) =>
		edit<ToonCredits>(credits, (file) => fn(getAt(file, path) as CreditEntry[], file), key);

	function moveEntry(from: number, to: number) {
		if (to < 0 || to >= list.length) return;
		change((l) => move(l, from, to));
	}
	function addPerson(id: string) {
		change((l) => l.push({ person: id, roles: [] }));
	}
	function createPerson(name: string) {
		const registry = peopleOf(people);
		if (!registry) return;
		const person = newPerson(name, registry);
		edit<{ people: any[] }>(people, (file) => file.people.push(person));
		addPerson(person.id);
	}
	function addRole(i: number, role: string) {
		change((l) => l[i].roles.push({ role }));
	}
	function createRole(i: number, label: LocaleText, category: RoleCategory) {
		const role = newRole(label, category, [...roleMap.keys()]);
		change((l, file) => {
			file.roles = [...(file.roles ?? []), role];
			l[i].roles.push({ role: role.id });
		});
	}
</script>

<div class="entries" data-testid={testid}>
	<ol use:sortable={{ onmove: moveEntry }}>
		{#each list as entry, i (entry.person + ':' + i)}
			{@const person = peopleMap.get(entry.person)}
			<li
				data-sort-row
				data-testid="{testid}-entry"
				data-person={entry.person}
			>
				<div class="line">
					<SortGrip label={m['creator.sort_move']({ name: person?.name ?? entry.person })} title={m['creator.sort_hint']()} />
					{#if person}
						<button type="button" class="name" onclick={() => onperson?.(entry.person)}>{person.name}</button>
						{#if person.merged_into}<span class="bad" title={m['creator.credits_merged_into']({ id: person.merged_into })}>({person.merged_into})</span>{/if}
					{:else}
						<span class="name bad">? {entry.person}</span>
					{/if}
					<span class="chips">
						{#each entry.roles as r, k (k)}
							{@const role = roleMap.get(r.role)}
							<span class="sc-chip-control is-filled" class:is-error={!role} class:is-draft={role?.custom}>
								<button type="button" onclick={() => (open = open === `${i}:${k}` ? null : `${i}:${k}`)} title={m['creator.credits_role_details']()}
									>{roleText(r, role, lang)}</button
								><button type="button" aria-label={m['creator.credits_remove_role']()} onclick={() => change((l) => l[i].roles.splice(k, 1))}><CloseIcon /></button>
							</span>
						{/each}
						<RolePicker
							roles={roleList}
							{languages}
							has={new Set(entry.roles.map((r) => r.role))}
							testid="{testid}-role-picker"
							onpick={(id) => addRole(i, id)}
							oncreate={(label, category) => createRole(i, label, category)}
						/>
					</span>
					<button type="button" class="ghost remove" aria-label={m['creator.credits_remove_person']()} title={m['creator.credits_remove_person']()} onclick={() => change((l) => l.splice(i, 1))}
						><CloseIcon /></button
					>
				</div>
				{#each entry.roles as r, k (k)}
					{#if open === `${i}:${k}`}
						<div class="details" data-testid="{testid}-role-details">
							<LocaleMapField
								value={r.detail}
								{languages}
								label={m['creator.credits_detail']()}
								onchange={(v) =>
									change((l) => {
										const empty = !Object.values(v).some((t) => t.trim());
										if (empty) delete l[i].roles[k].detail;
										else l[i].roles[k].detail = v;
									}, `detail:${i}:${k}`)}
							/>
							<label>
								<span class="hint">{m['creator.credits_language']()}</span>
								<select
									value={r.language ?? ''}
									onchange={(e) =>
										change((l) => {
											const v = e.currentTarget.value;
											if (v) l[i].roles[k].language = v as Language;
											else delete l[i].roles[k].language;
										})}
								>
									<option value="">–</option>
									{#each Object.values(Language) as l (l)}<option value={l}>{l}</option>{/each}
								</select>
							</label>
						</div>
					{/if}
				{/each}

			</li>
		{/each}
	</ol>
	<PersonPicker people={peopleOf(people)?.people ?? []} {listed} testid="{testid}-person-picker" onpick={addPerson} oncreate={createPerson} />
</div>

<style>
	.entries {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	li {
		border: 1px solid var(--border-subtle);
		padding: 0.25rem 0.4rem;
		background: var(--paper);
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.line {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
	}
	.name {
		all: unset;
		display: inline-flex;
		align-items: center;
		min-height: var(--control-height);
		cursor: pointer;
		min-width: 11em;
	}
	.name:hover {
		text-decoration: underline;
	}
	.bad {
		color: var(--error);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		flex: 1;
	}
	/* a role = a filled chip with buttons (tools.css .sc-chip-control.is-filled; a custom role .is-draft, unknown .is-error) */
	.remove {
		font-size: var(--type-caption-size);
		padding: 0 0.35rem;
	}
	.details {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		flex-wrap: wrap;
		padding: 0.3rem;
		background: var(--surface-panel);
	}
	.details label {
		display: flex;
		flex-direction: column;
	}
</style>
