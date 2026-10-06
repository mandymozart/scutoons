<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { RECURRENCE_UNITS, isRecurrenceInterval, placementRespawnProblem, repeatAsRespawn, respawnAsRepeat, respawnRange, type Path, type Repeat, type RecurrenceInterval, type RecurrenceUnit } from '@scutoons/content';
	import RepeatsField from '$lib/components/fields/repeats-field.svelte';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import { intervalLabel } from '$lib/editor/respawn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

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
		/** the slice's text boxes (a showTextBox tap event reveals one) */
		dialogs?: { id: string; label: string }[];
		/** "+ New text box" for tap event `index` */
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
	const UNIT_MINUTES: Record<RecurrenceUnit, number> = { minutes: 1, hours: 60, days: 1440 };
	/** the item's rule: undefined = catalog not loaded, null = the item never reappears */
	const rule = $derived(def ? (def.respawn ?? null) : undefined);
	const range = $derived(rule ? respawnRange(rule) : null);
	/** the units a placement can use within the range (n ≥ 1 whole) */
	const units = $derived(
		range ? RECURRENCE_UNITS.filter((u) => Math.ceil(Math.max(range.min, 1) / UNIT_MINUTES[u]) * UNIT_MINUTES[u] <= range.max) : []
	);
	const choice = $derived<RecurrenceInterval | 'never' | undefined>(placed?.respawn);
	const problem = $derived(rule === undefined ? null : placementRespawnProblem(choice, rule));
	const bounds = (u: RecurrenceUnit) => ({
		min: range ? Math.max(1, Math.ceil(Math.max(range.min, 1) / UNIT_MINUTES[u])) : 1,
		max: range ? Math.floor(range.max / UNIT_MINUTES[u]) : 1
	});
	// the pickup in the one "Repeats" field (recurrence.ts): never = once for good, visit = once per page view, n …
	const pickupRepeat = $derived<Repeat | undefined>(
		choice === undefined ? undefined : choice === 'never' ? { mode: 'once' } : isRecurrenceInterval(choice) ? respawnAsRepeat(choice) : (choice as unknown as Repeat)
	);
	const setPickup = (next: Repeat | undefined) => doc.set(p('respawn'), next === undefined ? undefined : (repeatAsRespawn(next) ?? undefined));
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
		<ItemPickerSheet bind:open={picking} title={m['creator.slice_item_change']()} onpick={(item) => doc.set(p('item'), item.id)} />

		<div class="sc-fields" data-cols="2" data-density="panel">
			<PropField {doc} path={p('rect', 'x')} label="X %" type="number" required />
			<PropField {doc} path={p('rect', 'y')} label="Y %" type="number" required />
			<PropField {doc} path={p('rect', 'width')} label={m['creator.field_width']()} type="number" required />
			<PropField {doc} path={p('rotation')} label={m['creator.field_rotation']()} type="number" step={1} placeholder="0" />
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
					<RepeatsField
						id="respawn-{path.join('-')}"
						testid="item-respawn"
						value={pickupRepeat}
						onchange={setPickup}
						pickup={{ defaultLabel: m['creator.respawn_item_default']({ every: intervalLabel(rule.default) }), view: range?.min === 0, units, bounds }}
					/>
				</div>
				<p class="hint">{m['creator.respawn_item_range']({ min: range && range.min === 0 ? intervalLabel('visit') : intervalLabel({ minutes: Math.max(1, range?.min ?? 1) }), max: intervalLabel({ minutes: range?.max ?? 1 }) })}</p>
				{#if problem}<p class="warn" data-testid="item-respawn-problem">{problem}</p>{/if}
			{/if}
			<p class="hint">{m['creator.slice_item_hint_pickup']()}</p>
			{#if def && !def.grantable?.includes('story')}
				<p class="warn" data-testid="item-not-grantable">{m['creator.slice_item_not_grantable']()}</p>
			{/if}
		{:else if action === 'events'}
			<div class="sc-fields" data-cols="2" data-density="panel">
				<RepeatsField id="repeat-{path.join('-')}" testid="item-repeat" value={placed.repeat} onchange={(next) => doc.set(p('repeat'), next)} />
			</div>
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
