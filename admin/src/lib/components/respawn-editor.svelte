<script lang="ts">
	import { Badge, Button, DataTable, type Column } from '@scutoons/ui';
	import { RESPAWN_UNITS, intervalMinutes, intervalParts, itemRespawnProblems, type ItemRespawn, type RespawnInterval, type RespawnUnit } from '@scutoons/content';
	import type { AdminRespawn } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * When picked-up placed items reappear (agents/ITEMS.md "Placed items" → "Respawn"), per catalog item: never (a
	 * one-time find) or a default interval with the range a placement may choose (min … max). Units: every visit
	 * (reload), minutes, hours, days. Stored over server config/items.json as the setting `item_respawn`.
	 */
	let { names }: { names: Record<string, string> } = $props();

	let data = $state<AdminRespawn | null>(null);
	let error = $state<string | null>(null);
	let editing = $state<string | null>(null);
	let busy = $state(false);

	type Row = Record<string, any>;
	type Kind = 'visit' | RespawnUnit;
	interface Field {
		kind: Kind;
		n: number;
	}
	let can = $state(false);
	let draft = $state<{ default: Field; min: Field; max: Field }>({ default: { kind: 'hours', n: 6 }, min: { kind: 'hours', n: 1 }, max: { kind: 'days', n: 7 } });

	$effect(() => {
		admin
			.respawn()
			.then((r) => (data = r))
			.catch((e) => (error = errorText(e)));
	});

	const toField = (i: RespawnInterval | undefined, fallback: Field): Field => {
		if (!i) return fallback;
		if (i === 'visit') return { kind: 'visit', n: 1 };
		const { unit, n } = intervalParts(i);
		return { kind: unit as RespawnUnit, n };
	};
	const toInterval = (f: Field): RespawnInterval => (f.kind === 'visit' ? 'visit' : ({ [f.kind]: Math.max(1, Math.floor(f.n)) } as RespawnInterval));

	function label(i: RespawnInterval | undefined): string {
		if (!i) return '–';
		const minutes = intervalMinutes(i) ?? 0;
		if (minutes === 0) return m['catalog.respawn_visit']();
		const { unit, n } = intervalParts(i);
		return `${n} ${m[`catalog.respawn_unit_${unit as RespawnUnit}`]()}`;
	}
	function summary(rule: ItemRespawn | null): string {
		if (!rule) return m['catalog.respawn_never']();
		return `${label(rule.default)} (${label(rule.min ?? rule.default)} … ${rule.max ? label(rule.max) : '∞'})`;
	}

	function edit(id: string) {
		const row = data?.items.find((r) => r.id === id);
		const rule = row?.effective ?? null;
		can = !!rule;
		draft = {
			default: toField(rule?.default, { kind: 'hours', n: 6 }),
			min: toField(rule?.min ?? rule?.default, { kind: 'hours', n: 1 }),
			max: toField(rule?.max, { kind: 'days', n: 7 })
		};
		editing = id;
	}

	const rule = $derived<ItemRespawn | null>(can ? { default: toInterval(draft.default), min: toInterval(draft.min), max: toInterval(draft.max) } : null);
	const problems = $derived(itemRespawnProblems(rule));

	async function save(change: { respawn: ItemRespawn | null } | { reset: true }) {
		if (!editing) return;
		busy = true;
		error = null;
		try {
			data = await admin.saveRespawn(editing, change);
			editing = null;
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'id', label: 'id' },
		{ id: 'name', label: m['catalog.name']() },
		{ id: 'effective', label: m['catalog.respawn']() },
		{ id: 'edit', label: '' }
	]);
</script>

{#snippet field(key: 'default' | 'min' | 'max', title: string)}
	<div class="field">
		<span class="label">{title}</span>
		<span class="inputs">
			<select bind:value={draft[key].kind} aria-label={title} data-testid="respawn-{key}-unit">
				<option value="visit">{m['catalog.respawn_visit']()}</option>
				{#each RESPAWN_UNITS as u (u)}<option value={u}>{m[`catalog.respawn_unit_${u}`]()}</option>{/each}
			</select>
			{#if draft[key].kind !== 'visit'}
				<input type="number" min="1" step="1" bind:value={draft[key].n} aria-label="{title} n" data-testid="respawn-{key}-n" />
			{/if}
		</span>
	</div>
{/snippet}

<p class="muted">{m['catalog.respawn_hint']()}</p>
{#if error}<p class="error-box" role="alert">{error}</p>{/if}
{#if data}
	<DataTable {columns} rows={data.items as Row[]} rowKey={(r) => r.id} labels={tableLabels()}>
		{#snippet cell(row, column)}
			{#if column.id === 'name'}{names[row.id] ?? row.id}
			{:else if column.id === 'effective'}
				<span class="badges">
					{summary(row.effective)}
					{#if row.overridden}<Badge tone="warning">{m['catalog.respawn_overridden']()}</Badge>{/if}
				</span>
			{:else if column.id === 'edit'}
				<Button size="sm" variant="secondary" onclick={() => edit(row.id)} data-testid="respawn-edit-{row.id}">{m['catalog.respawn_edit']()}</Button>
			{:else}{row.id}{/if}
		{/snippet}
	</DataTable>
{/if}

{#if editing}
	<section class="editor" data-testid="respawn-editor">
		<h3>{m['catalog.respawn']()} · {names[editing] ?? editing}</h3>
		<label class="check"><input type="checkbox" bind:checked={can} data-testid="respawn-can" /> {m['catalog.respawn_can']()}</label>
		{#if can}
			<div class="grid">
				{@render field('default', m['catalog.respawn_default']())}
				{@render field('min', m['catalog.respawn_min']())}
				{@render field('max', m['catalog.respawn_max']())}
			</div>
			{#each problems as p (p)}<p class="error-box">{p}</p>{/each}
		{/if}
		<span class="actions">
			<Button size="sm" onclick={() => save({ respawn: rule })} disabled={busy || problems.length > 0} data-testid="respawn-save">{m['catalog.respawn_save']()}</Button>
			{#if data?.items.find((r: Row) => r.id === editing)?.overridden}
				<Button size="sm" variant="secondary" onclick={() => save({ reset: true })} disabled={busy}>{m['catalog.respawn_reset']()}</Button>
			{/if}
			<Button size="sm" variant="clear" onclick={() => (editing = null)}>{m['catalog.respawn_cancel']()}</Button>
		</span>
	</section>
{/if}

<style>
	.badges {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-025) var(--space-05);
	}
	.editor {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-top: var(--space-1);
		padding: var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
	}
	.editor h3 {
		margin: 0;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
		gap: var(--space-1);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.inputs {
		display: flex;
		gap: var(--space-05);
	}
	.inputs input {
		width: 6rem;
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
</style>
