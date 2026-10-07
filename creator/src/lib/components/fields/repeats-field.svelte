<script lang="ts" module>
	import type { RecurrenceUnit } from '@scutoons/content';

	/** a placed item's pickup: the catalog's rule decides what a placement may choose (item-inspector.svelte) */
	export interface PickupChoices {
		/** "Item default (every 4 h)" */
		defaultLabel: string;
		/** the item may come back every page view */
		view: boolean;
		/** the timed units within the item's range */
		units: RecurrenceUnit[];
		/** n's bounds per unit */
		bounds: (unit: RecurrenceUnit) => { min: number; max: number };
	}
</script>

<script lang="ts">
	import { RECURRENCE_UNITS, intervalParts, repeatOf, repeatProblem, type Repeat, type TimedInterval } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Repeats" (agents/EXPRESSIONS.md §5 – one recurrence model, one field): how often a slice trigger's / a placed
	 * item's tap events run – every time (the default: nothing stored), once per page view, once for good (the reader's
	 * save game), every n minutes / hours / days – or, with `pickup`, when a picked-up item is back (the item's default,
	 * once for good, every page view, every n … within the catalog's range). A select, and the number when it has one.
	 * The cells join the parent's field grid.
	 */
	let {
		value,
		onchange,
		pickup,
		id,
		testid = 'repeats'
	}: {
		/** the rule; undefined = the default (every time; a pickup: the item's default) */
		value: Repeat | undefined;
		onchange: (next: Repeat | undefined) => void;
		pickup?: PickupChoices;
		/** the select's id (labels) */
		id: string;
		testid?: string;
	} = $props();

	type Kind = 'default' | 'every' | 'view' | 'once' | RecurrenceUnit;
	const rule = $derived(value === undefined ? undefined : repeatOf(value));
	const broken = $derived(value !== undefined && !!repeatProblem(value));
	const kind = $derived<Kind>(
		rule === undefined || broken ? (pickup ? 'default' : 'every') : rule.mode === 'interval' ? intervalParts(rule.every!).unit as RecurrenceUnit : rule.mode
	);
	const units = $derived(pickup ? pickup.units : [...RECURRENCE_UNITS]);
	/** n's bounds: the item's range (pickup), else 1 … a year */
	const bounds = (u: RecurrenceUnit) => pickup?.bounds(u) ?? { min: 1, max: u === 'minutes' ? 525600 : u === 'hours' ? 8760 : 365 };
	const n = $derived(rule?.mode === 'interval' && rule.every ? intervalParts(rule.every).n : 1);

	function choose(next: string) {
		if (next === 'default' || (next === 'every' && !pickup)) return onchange(undefined);
		if (next === 'view' || next === 'once') return onchange({ mode: next });
		const u = next as RecurrenceUnit;
		const b = bounds(u);
		onchange({ mode: 'interval', every: { [u]: Math.min(b.max, Math.max(b.min, 1)) } as TimedInterval });
	}
	/**
	 * a typed number outside the bounds (owner 2026-10-08: "no warning that it can't be set below the catalog's minimum?"):
	 * not stored – a warning says why and what is allowed; leaving the field shows the stored value again
	 */
	let outside = $state(false);
	function setN(raw: string) {
		if (kind === 'default' || kind === 'every' || kind === 'view' || kind === 'once') return;
		const b = bounds(kind);
		const count = Math.floor(Number(raw));
		outside = raw.trim() !== '' && !(count >= b.min && count <= b.max);
		if (!outside && raw.trim() !== '') onchange({ mode: 'interval', every: { [kind]: count } as TimedInterval });
	}
	function leaveN(input: HTMLInputElement) {
		if (!outside) return;
		input.value = String(n);
		outside = false;
	}
	const unitWord = (u: RecurrenceUnit) => m[`creator.repeat_every_${u}_n`]();
</script>

<div class="field" data-testid={testid}>
	<label for={id}>{pickup ? m['creator.slice_item_pickup_mode']() : m['creator.repeat']()}</label>
	<select {id} value={kind} onchange={(e) => choose(e.currentTarget.value)} data-testid="{testid}-mode">
		{#if pickup}
			<option value="default">{pickup.defaultLabel}</option>
		{:else}
			<option value="every">{m['creator.repeat_every']()}</option>
		{/if}
		{#if !pickup || pickup.view}<option value="view">{m['creator.repeat_view']()}</option>{/if}
		<option value="once">{m['creator.repeat_once']()}</option>
		{#each units as u (u)}<option value={u}>{unitWord(u)}</option>{/each}
		{#if (kind !== 'default' && kind !== 'every' && kind !== 'view' && kind !== 'once' && !units.includes(kind)) || broken}
			<option value={kind}>{JSON.stringify(value)} (?)</option>
		{/if}
	</select>
</div>
{#if kind !== 'default' && kind !== 'every' && kind !== 'view' && kind !== 'once'}
	{@const b = bounds(kind)}
	<div class="field">
		<label for="{id}-n">{m[`creator.respawn_unit_${kind}`]()}{pickup ? ` (${b.min} … ${b.max})` : ''}</label>
		<input
			id="{id}-n"
			type="number"
			step="1"
			min={b.min}
			max={b.max}
			value={n}
			oninput={(e) => setN(e.currentTarget.value)}
			onblur={(e) => leaveN(e.currentTarget)}
			aria-invalid={outside}
			aria-describedby={outside ? `${id}-range` : undefined}
			data-testid="{testid}-n"
		/>
	</div>
	{#if outside}
		<p class="warn" id="{id}-range" role="alert" data-testid="{testid}-range">
			{pickup
				? m['creator.repeat_outside_catalog']({ min: b.min, max: b.max, unit: m[`creator.respawn_unit_${kind}`](), stored: n })
				: m['creator.repeat_outside']({ min: b.min, max: b.max, stored: n })}
		</p>
	{/if}
{/if}

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	/* laid out like a PropField */
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
</style>
