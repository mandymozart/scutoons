<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { tick } from 'svelte';
	import { getAt, Language, peopleIndex, roleIndex, roleText, type CreditEntry, type LocaleText, type Path, type RoleCategory, type ToonCredits } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { creditsOf, edit, move, newPerson, newRole, peopleOf, rolesOf } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PersonPicker from '$lib/components/credits/person-picker.svelte';
	import RolePicker from '$lib/components/credits/role-picker.svelte';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * An ordered list of `{ person, roles }` of a section (toon sections have two: lead + entries). Reorder by dragging a
	 * row or with the ⋮⋮ handle focused + ArrowUp / ArrowDown; roles are chips (click = detail / language, × = remove).
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
	let dragFrom = $state<number | null>(null);
	let dropAt = $state<number | null>(null);
	let root = $state<HTMLElement>();

	const change = (fn: (list: CreditEntry[], file: ToonCredits) => void, key?: string) =>
		edit<ToonCredits>(credits, (file) => fn(getAt(file, path) as CreditEntry[], file), key);

	async function moveEntry(from: number, to: number, focus = false) {
		if (to < 0 || to >= list.length) return;
		change((l) => move(l, from, to));
		if (focus) {
			await tick();
			root?.querySelectorAll<HTMLElement>('[data-handle]')[to]?.focus();
		}
	}
	function onHandleKey(event: KeyboardEvent, i: number) {
		if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
			event.preventDefault();
			moveEntry(i, i + (event.key === 'ArrowUp' ? -1 : 1), true);
		}
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

<div class="entries" bind:this={root} data-testid={testid}>
	<ol>
		{#each list as entry, i (entry.person + ':' + i)}
			{@const person = peopleMap.get(entry.person)}
			<li
				class:drop={dropAt === i && dragFrom !== null && dragFrom !== i}
				class:dragging={dragFrom === i}
				draggable="true"
				ondragstart={(e) => {
					dragFrom = i;
					e.dataTransfer?.setData('text/plain', String(i));
				}}
				ondragover={(e) => {
					if (dragFrom === null) return;
					e.preventDefault();
					dropAt = i;
				}}
				ondrop={(e) => {
					e.preventDefault();
					if (dragFrom !== null) moveEntry(dragFrom, i);
					dragFrom = dropAt = null;
				}}
				ondragend={() => (dragFrom = dropAt = null)}
				data-testid="{testid}-entry"
				data-person={entry.person}
			>
				<div class="line">
					<button
						type="button"
						class="handle"
						data-handle
						aria-label={m['creator.credits_move']({ name: person?.name ?? entry.person })}
						title={m['creator.credits_move_hint']()}
						onkeydown={(e) => onHandleKey(e, i)}>⋮⋮</button
					>
					{#if person}
						<button type="button" class="name" onclick={() => onperson?.(entry.person)}>{person.name}</button>
						{#if person.merged_into}<span class="bad" title={m['creator.credits_merged_into']({ id: person.merged_into })}>({person.merged_into})</span>{/if}
					{:else}
						<span class="name bad">? {entry.person}</span>
					{/if}
					<span class="chips">
						{#each entry.roles as r, k (k)}
							{@const role = roleMap.get(r.role)}
							<span class="chip" class:unknown={!role} class:custom={role?.custom}>
								<button type="button" class="chip-label" onclick={() => (open = open === `${i}:${k}` ? null : `${i}:${k}`)} title={m['creator.credits_role_details']()}
									>{roleText(r, role, lang)}</button
								><button type="button" class="chip-x" aria-label={m['creator.credits_remove_role']()} onclick={() => change((l) => l[i].roles.splice(k, 1))}><CloseIcon /></button>
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
	li.dragging {
		opacity: 0.4;
	}
	li.drop {
		border-top: 3px solid var(--primary-cyan);
	}
	.line {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
	}
	.handle {
		all: unset;
		display: inline-flex;
		align-items: center;
		min-height: var(--control-height);
		cursor: grab;
		padding: 0 0.25rem;
		color: var(--text-muted);
		letter-spacing: -2px;
	}
	.handle:focus-visible {
		outline: 2px solid var(--ink);
		color: var(--ink);
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
	.bad,
	.unknown .chip-label {
		color: var(--error);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		flex: 1;
	}
	/* a role = a filled tag (tokens.css --badge-*): neutral, a custom role = draft */
	.chip {
		display: inline-flex;
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
	}
	.chip.custom {
		background: var(--badge-draft);
		color: var(--badge-draft-text);
	}
	.chip-label,
	.chip-x {
		all: unset;
		display: inline-flex;
		align-items: center;
		box-sizing: border-box;
		height: var(--control-height);
		cursor: pointer;
		padding: 0 var(--space-05);
		line-height: var(--type-button-line);
		white-space: nowrap;
	}
	.chip-x {
		border-left: var(--border-width-hairline) solid currentColor;
	}
	.chip-label:hover,
	.chip-x:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
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
