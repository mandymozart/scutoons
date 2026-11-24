<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { ROLE_CATEGORIES, RoleCategory, pickText, type CreditRole, type LocaleText } from '@scutoons/content';
	import { Chip, Select } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

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
	<button type="button" class="ghost mini" onclick={start} data-testid="{testid}-open"><Icon name="plus" /> {m['creator.credits_role']()}</button>
{:else}
	<div class="role-picker" data-testid={testid}>
		<div class="row">
			<input bind:this={input} type="search" placeholder={m['creator.credits_search_roles']()} bind:value={query} {onkeydown} data-testid="{testid}-input" />
			<button type="button" class="ghost" onclick={() => (open = false)} aria-label={m['creator.close']()}><CloseIcon /></button>
		</div>
		{#if !creating}
			<div class="list">
				{#each groups as group (group.category)}
					<div class="group">
						<span class="cat">{group.category}</span>
						{#each group.roles as role (role.id)}
							<Chip label="{pickText(role.label, lang) || role.id}{role.custom ? ` · ${m['creator.credits_custom']()}` : ''}" on={has.has(role.id)} onclick={() => pick(role.id)} testid="{testid}-role" data-role={role.id} />
						{/each}
					</div>
				{/each}
			</div>
			<button type="button" class="secondary mini" onclick={startCreate} data-testid="{testid}-new"><Icon name="plus" /> {m['creator.credits_new_custom_role']()}</button>
		{:else}
			<div class="create" data-testid="{testid}-create">
				<LocaleMapField value={label} {languages} label={m['creator.credits_label']()} required onchange={(v) => (label = v)} testid="{testid}-label" />
				<label>
					<span class="cat">{m['creator.credits_category']()}</span>
					<Select label={m['creator.credits_category']()} value={category} onchange={(v) => (category = v as RoleCategory)} options={ROLE_CATEGORIES.map((c) => ({ value: c, label: c }))} data-testid="{testid}-category" />
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
		border: 2px solid var(--tool-text);
		background: var(--tool-popover);
		text-shadow: var(--tool-halo);
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
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--tool-muted);
		min-width: 6.5rem;
	}
	.create {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.mini {
		font-size: var(--type-caption-size);
		padding: 0 0.4rem;
	}
</style>
