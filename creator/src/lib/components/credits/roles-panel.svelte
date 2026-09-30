<script lang="ts">
	import { forEachEntryList, ROLE_CATEGORIES, RoleCategory, type CreditRole, type RoleVocabulary, type ToonCredits } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { creditsOf, edit, newRole, rolesOf } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from './locale-map-field.svelte';

	/** The toon's custom roles (credits.json `roles`) and the shared vocabulary (scutoons/_/roles.json). */
	let { credits, roles, languages }: { credits: ContentDoc; roles: ContentDoc; languages: string[] } = $props();

	let query = $state('');
	const custom = $derived(creditsOf(credits)?.roles ?? []);
	const vocabulary = $derived(rolesOf(roles)?.roles ?? []);
	const usage = $derived.by(() => {
		const counts = new Map<string, number>();
		forEachEntryList(creditsOf(credits), (list) => list.forEach((e) => e.roles.forEach((r) => counts.set(r.role, (counts.get(r.role) ?? 0) + 1))));
		return counts;
	});
	const matches = (r: CreditRole) => !query.trim() || [r.id, ...Object.values(r.label ?? {})].some((t) => t?.toLowerCase().includes(query.trim().toLowerCase()));
	const missing = (r: CreditRole) => languages.filter((l) => !r.label?.[l]?.trim());
	const allIds = $derived([...vocabulary, ...custom].map((r) => r.id));

	const editCustom = (i: number, fn: (r: CreditRole) => void, key?: string) => edit<ToonCredits>(credits, (f) => fn(f.roles![i]), key);
	const editGlobal = (i: number, fn: (r: CreditRole) => void, key?: string) => edit<RoleVocabulary>(roles, (f) => fn(f.roles[i]), key);
</script>

{#snippet table(list: CreditRole[], scope: 'custom' | 'global')}
	<table>
		<thead>
			<tr><th>id</th><th>{m['creator.credits_category']()}</th><th>{m['creator.credits_label']()}</th><th>{m['creator.credits_used']()}</th><th></th></tr>
		</thead>
		<tbody>
			{#each list as role, i (role.id)}
				{#if matches(role)}
					{@const miss = missing(role)}
					<tr data-testid="{scope}-role" data-role={role.id}>
						<td><code>{role.id}</code>{#if miss.length}<br /><span class="warn">{m['creator.missing']()}: {miss.join(', ')}</span>{/if}</td>
						<td>
							<select
								value={role.category}
								onchange={(e) => (scope === 'custom' ? editCustom : editGlobal)(i, (r) => (r.category = e.currentTarget.value as RoleCategory))}
							>
								{#each ROLE_CATEGORIES as c (c)}<option value={c}>{c}</option>{/each}
							</select>
						</td>
						<td>
							<LocaleMapField value={role.label} {languages} label="" required onchange={(v) => (scope === 'custom' ? editCustom : editGlobal)(i, (r) => (r.label = v), `label:${role.id}`)} />
						</td>
						<td>{usage.get(role.id) ?? 0}</td>
						<td>
							{#if !(usage.get(role.id) ?? 0) && scope === 'custom'}
								<button type="button" class="ghost" onclick={() => edit<ToonCredits>(credits, (f) => f.roles!.splice(i, 1))}>{m['creator.credits_remove']()}</button>
							{/if}
						</td>
					</tr>
				{/if}
			{/each}
		</tbody>
	</table>
{/snippet}

<div class="roles" data-testid="roles-panel">
	<input type="search" placeholder={m['creator.credits_search_roles']()} bind:value={query} />

	<h3>{m['creator.credits_custom_roles']()} ({custom.length})</h3>
	{@render table(custom, 'custom')}
	<button type="button" class="ghost" onclick={() => edit<ToonCredits>(credits, (f) => (f.roles = [...(f.roles ?? []), newRole({ [languages[0]]: 'New role' }, RoleCategory.OTHER, allIds)]))}
		>+ {m['creator.credits_new_custom_role']()}</button
	>

	<h3>{m['creator.credits_vocabulary']()} ({vocabulary.length})</h3>
	{@render table(vocabulary, 'global')}
	<button type="button" class="ghost" onclick={() => edit<RoleVocabulary>(roles, (f) => f.roles.push(newRole({ [languages[0]]: 'New role' }, RoleCategory.OTHER, allIds)))}
		>+ {m['creator.credits_new_global_role']()}</button
	>
</div>

<style>
	.roles {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	h3 {
		margin: 0.75rem 0 0;
		font-size: 14px;
	}
	table {
		border-collapse: collapse;
		width: 100%;
		font-size: 12px;
	}
	th,
	td {
		border-bottom: 1px solid var(--border-subtle);
		padding: 0.25rem;
		text-align: left;
		vertical-align: top;
	}
	.warn {
		color: var(--warn);
		font-size: 10px;
	}
</style>
