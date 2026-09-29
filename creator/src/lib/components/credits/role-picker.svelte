<script lang="ts">
	import { ROLE_CATEGORIES, RoleCategory, pickText, type CreditRole, type LocaleText } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from './locale-map-field.svelte';

	/**
	 * "+ Role": searchable role list grouped by category (the shared vocabulary + the toon's custom roles), and a form
	 * for a new custom role of this toon (a label per toon language).
	 */
	let {
		roles,
		languages,
		has = new Set<string>(),
		testid = 'role-picker',
		onpick,
		oncreate
	}: {
		roles: (CreditRole & { custom?: boolean })[];
		languages: string[];
		/** roles the entry already has (marked) */
		has?: Set<string>;
		testid?: string;
		onpick: (id: string) => void;
		oncreate: (label: LocaleText, category: RoleCategory) => void;
	} = $props();

	let open = $state(false);
	let query = $state('');
	let creating = $state(false);
	let label = $state<LocaleText>({});
	let category = $state<RoleCategory>(RoleCategory.OTHER);
	let input = $state<HTMLInputElement>();
	const lang = $derived(languages[0] ?? 'en');

	const matches = (role: CreditRole) => {
		const q = query.trim().toLowerCase();
		if (!q) return true;
		return [role.id, ...Object.values(role.label ?? {})].some((t) => t?.toLowerCase().includes(q));
	};
	const groups = $derived(
		ROLE_CATEGORIES.map((c) => ({
			category: c,
			roles: roles.filter((r) => (r.category ?? RoleCategory.OTHER) === c && matches(r)).sort((a, b) => pickText(a.label, lang).localeCompare(pickText(b.label, lang)))
		})).filter((g) => g.roles.length)
	);
	const flat = $derived(groups.flatMap((g) => g.roles));

	function start() {
		open = true;
		creating = false;
		query = '';
		queueMicrotask(() => input?.focus());
	}
	function pick(id: string) {
		onpick(id);
		open = false;
	}
	function startCreate() {
		creating = true;
		label = Object.fromEntries(languages.map((l, i) => [l, i === 0 ? query.trim() : '']));
		category = RoleCategory.OTHER;
	}
	function create() {
		if (!Object.values(label).some((t) => t.trim())) return;
		oncreate(label, category);
		open = false;
	}
	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			open = false;
		} else if (event.key === 'Enter' && flat.length) {
			event.preventDefault();
			pick(flat[0].id);
		}
	}
</script>

{#if !open}
	<button type="button" class="ghost mini" onclick={start} data-testid="{testid}-open">+ {m['creator.credits_role']()}</button>
{:else}
	<div class="role-picker" data-testid={testid}>
		<div class="row">
			<input bind:this={input} type="search" placeholder={m['creator.credits_search_roles']()} bind:value={query} {onkeydown} data-testid="{testid}-input" />
			<button type="button" class="ghost" onclick={() => (open = false)} aria-label="Close">✕</button>
		</div>
		{#if !creating}
			<div class="list">
				{#each groups as group (group.category)}
					<div class="group">
						<span class="cat">{group.category}</span>
						{#each group.roles as role (role.id)}
							<button type="button" class="chip" class:has={has.has(role.id)} class:custom={role.custom} onclick={() => pick(role.id)} data-testid="{testid}-role" data-role={role.id}>
								{pickText(role.label, lang) || role.id}{#if role.custom}<span class="tag">{m['creator.credits_custom']()}</span>{/if}
							</button>
						{/each}
					</div>
				{:else}
					<p class="hint">{m['creator.credits_no_roles']()}</p>
				{/each}
			</div>
			<button type="button" class="secondary mini" onclick={startCreate} data-testid="{testid}-new">+ {m['creator.credits_new_custom_role']()}</button>
		{:else}
			<div class="create" data-testid="{testid}-create">
				<LocaleMapField value={label} {languages} label={m['creator.credits_label']()} required onchange={(v) => (label = v)} testid="{testid}-label" />
				<label>
					<span class="cat">{m['creator.credits_category']()}</span>
					<select bind:value={category} data-testid="{testid}-category">
						{#each ROLE_CATEGORIES as c (c)}<option value={c}>{c}</option>{/each}
					</select>
				</label>
				<div class="row">
					<button type="button" onclick={create} disabled={!Object.values(label).some((t) => t.trim())} data-testid="{testid}-create-submit">{m['creator.credits_create_role']()}</button>
					<button type="button" class="ghost" onclick={() => (creating = false)}>{m['creator.cancel']()}</button>
				</div>
			</div>
		{/if}
	</div>
{/if}

<style>
	.role-picker {
		border: 2px solid var(--ink);
		background: var(--paper);
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin: 0.25rem 0;
		max-width: 40rem;
		flex-basis: 100%;
	}
	.row {
		display: flex;
		gap: 0.25rem;
		align-items: center;
	}
	.row input {
		flex: 1;
	}
	.list {
		max-height: 16rem;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.group {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		align-items: center;
	}
	.cat {
		font-size: 10px;
		text-transform: uppercase;
		color: var(--text-muted);
		min-width: 6.5rem;
	}
	.chip {
		font-size: 12px;
		padding: 0.1rem 0.4rem;
		background: var(--paper);
		color: var(--ink);
		border: 1px solid var(--border-muted);
	}
	.chip:hover {
		border-color: var(--ink);
	}
	.chip.has {
		background: var(--surface-hover);
	}
	.tag {
		font-size: 9px;
		margin-left: 0.3em;
		color: var(--text-muted);
	}
	.create {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.mini {
		font-size: 11px;
		padding: 0.1rem 0.4rem;
	}
</style>
