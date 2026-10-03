<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { RESPAWN_UNITS, intervalMinutes, isRespawnInterval, intervalParts, placementRespawnProblem, respawnRange, type Path, type RespawnInterval, type RespawnUnit } from '@scutoons/content';
	import ItemPickerSheet from './item-picker-sheet.svelte';
	import { intervalLabel } from '$lib/editor/respawn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import PropField from './prop-field.svelte';
	import VnEvents from './vn-events.svelte';
	import type { CharacterChoices } from './characters/character-ref-field.svelte';

	/**
	 * A catalog item placed on a slice (`Slice.items`): which item, where (x / y / width, rotation – also with the
	 * handles on the canvas) and what a tap does (`action`, the creator's choice: nothing, pick up, or VN events run like
	 * a click trigger's).
	 */
	let {
		doc,
		path,
		characters = [],
		choices,
		dialogs = [],
		oncreatebox,
		onremove
	}: {
		doc: ContentDoc;
		/** path of the placed item */
		path: Path;
		characters?: { name: string; label?: string }[];
		choices?: CharacterChoices;
		/** the slice's text boxes (an onomatopoeia tap event reveals one) */
		dialogs?: { id: string; label: string }[];
		/** "Create a text box for this" for tap event `index` */
		oncreatebox?: (index: number) => void;
		onremove?: () => void;
	} = $props();

	$effect(() => {
		loadCatalog();
	});

	const placed = $derived(doc.get(doc.primary, path));
	const def = $derived(itemCatalog.catalog?.items.find((i) => i.id === placed?.item));
	const action = $derived(placed?.action ?? 'none');
	const p = (...keys: (string | number)[]) => [...path, ...keys];
	let picking = $state(false);

	// ── respawn: the item's rule (catalog, admin → Catalog) and this placement's override within its range ──
	const UNIT_MINUTES: Record<RespawnUnit, number> = { minutes: 1, hours: 60, days: 1440 };
	/** the item's rule: undefined = catalog not loaded, null = the item never reappears */
	const rule = $derived(def ? (def.respawn ?? null) : undefined);
	const range = $derived(rule ? respawnRange(rule) : null);
	/** the units a placement can use within the range (n ≥ 1 whole) */
	const units = $derived(
		range ? RESPAWN_UNITS.filter((u) => Math.ceil(Math.max(range.min, 1) / UNIT_MINUTES[u]) * UNIT_MINUTES[u] <= range.max) : []
	);
	const choice = $derived<RespawnInterval | 'never' | undefined>(placed?.respawn);
	const choiceKind = $derived(
		choice === undefined ? 'default' : choice === 'never' || choice === 'visit' ? choice : isRespawnInterval(choice) ? intervalParts(choice).unit : 'invalid'
	);
	const problem = $derived(rule === undefined ? null : placementRespawnProblem(choice, rule));
	const bounds = (u: RespawnUnit) => ({
		min: range ? Math.max(1, Math.ceil(Math.max(range.min, 1) / UNIT_MINUTES[u])) : 1,
		max: range ? Math.floor(range.max / UNIT_MINUTES[u]) : 1
	});
	function setRespawnKind(kind: string) {
		let next: RespawnInterval | 'never' | undefined;
		if (kind === 'never' || kind === 'visit') next = kind;
		else if ((RESPAWN_UNITS as readonly string[]).includes(kind)) {
			const u = kind as RespawnUnit;
			// the item's default in this unit when it fits, else the shortest the range allows
			const d = rule ? (intervalMinutes(rule.default) ?? 0) : 0;
			const n = Math.min(bounds(u).max, Math.max(bounds(u).min, Math.round(d / UNIT_MINUTES[u])));
			next = { [u]: n } as RespawnInterval;
		}
		doc.setShared(p('respawn'), next);
	}
	function setRespawnCount(raw: string) {
		if (choiceKind === 'default' || choiceKind === 'never' || choiceKind === 'visit' || choiceKind === 'invalid') return;
		const n = Math.floor(Number(raw));
		if (n >= 1) doc.setShared(p('respawn'), { [choiceKind]: n });
	}
</script>

{#if placed}
	<div class="inspector" data-testid="item-inspector">
		<div class="row head">
			<span class="icon">{#if def?.icon}<img src={asset(def.icon)} alt="" />{/if}</span>
			<h3>{itemName(def, placed.item ?? '–')}</h3>
			{#if onremove}<button class="secondary" onclick={onremove}>{m['creator.remove']()}</button>{/if}
		</div>

		<div class="row">
			<code>{placed.item}</code>
			<button type="button" class="secondary" onclick={() => (picking = true)} data-testid="item-change">{m['creator.slice_item_change']()}</button>
		</div>
		<ItemPickerSheet bind:open={picking} title={m['creator.slice_item_change']()} onpick={(item) => doc.setShared(p('item'), item.id)} />

		<div class="sc-fields" data-cols="2" data-density="panel">
			<PropField {doc} path={p('rect', 'x')} label="x %" type="number" required />
			<PropField {doc} path={p('rect', 'y')} label="y %" type="number" required />
			<PropField {doc} path={p('rect', 'width')} label="width %" type="number" required />
			<PropField {doc} path={p('rotation')} label="rotation °" type="number" step={1} placeholder="0" />
		</div>

		<PropField
			{doc}
			path={p('action')}
			label={m['creator.slice_item_action']()}
			type="select"
			options={[
				{ value: 'pickup', label: m['creator.slice_item_action_pickup']() },
				{ value: 'events', label: m['creator.slice_item_action_events']() }
			]}
			placeholder={m['creator.slice_item_action_none']()}
		/>

		{#if action === 'pickup'}
			<PropField {doc} path={p('qty')} label={m['creator.slice_item_qty']()} type="number" step={1} min={1} placeholder="1" />
			{#if rule === null}
				<p class="hint" data-testid="item-respawn-never">{m['creator.respawn_item_never']()}</p>
			{:else if rule}
				<div class="sc-fields" data-cols="2" data-density="panel">
					<div class="field">
						<label for="respawn-{path.join('-')}">{m['creator.slice_item_pickup_mode']()}</label>
						<select id="respawn-{path.join('-')}" value={choiceKind} onchange={(e) => setRespawnKind(e.currentTarget.value)} data-testid="item-respawn">
							<option value="default">{m['creator.respawn_item_default']({ every: intervalLabel(rule.default) })}</option>
							<option value="never">{m['creator.respawn_never']()}</option>
							{#if range?.min === 0}<option value="visit">{intervalLabel('visit')}</option>{/if}
							{#each units as u (u)}<option value={u}>{m[`creator.respawn_every_${u}_n`]()}</option>{/each}
							{#if choiceKind === 'invalid' || (choiceKind !== 'default' && choiceKind !== 'never' && choiceKind !== 'visit' && !units.includes(choiceKind))}
								<option value={choiceKind}>{JSON.stringify(choice)} (?)</option>
							{/if}
						</select>
					</div>
					{#if choiceKind !== 'default' && choiceKind !== 'never' && choiceKind !== 'visit' && choiceKind !== 'invalid' && choice && typeof choice === 'object'}
						{@const b = bounds(choiceKind)}
						<div class="field">
							<label for="respawn-n-{path.join('-')}">{m[`creator.respawn_unit_${choiceKind}`]()} ({b.min} … {b.max})</label>
							<input id="respawn-n-{path.join('-')}" type="number" step="1" min={b.min} max={b.max} value={intervalParts(choice).n} oninput={(e) => setRespawnCount(e.currentTarget.value)} data-testid="item-respawn-n" />
						</div>
					{/if}
				</div>
				<p class="hint">{m['creator.respawn_item_range']({ min: range && range.min === 0 ? intervalLabel('visit') : intervalLabel({ minutes: Math.max(1, range?.min ?? 1) }), max: intervalLabel({ minutes: range?.max ?? 1 }) })}</p>
				{#if problem}<p class="warn" data-testid="item-respawn-problem">{problem}</p>{/if}
			{/if}
			<p class="hint">{m['creator.slice_item_hint_pickup']()}</p>
			{#if def && !def.grantable?.includes('story')}
				<p class="warn" data-testid="item-not-grantable">{m['creator.slice_item_not_grantable']()}</p>
			{/if}
		{:else if action === 'events'}
			<PropField {doc} path={p('once')} label={m['creator.slice_item_once']()} type="bool" placeholder="true" />
			<p class="hint">{m['creator.slice_item_hint_events']()}</p>
			<VnEvents {doc} eventsPath={p('events')} {characters} {choices} {dialogs} {oncreatebox} />
		{/if}
	</div>
{/if}

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	h3 {
		margin: 0;
		flex: 1;
		min-width: 0;
	}
	.icon {
		flex: none;
		width: var(--control-height);
		height: var(--control-height);
		display: inline-flex;
	}
	.icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	/* the respawn select (its own change handler) – laid out like a PropField */
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.warn {
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--state-warning);
	}
</style>
