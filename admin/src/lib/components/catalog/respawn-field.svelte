<script lang="ts">
	import { RESPAWN_UNITS, intervalParts, itemRespawnProblems, type ItemRespawn, type RespawnInterval, type RespawnUnit } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * When a picked-up placed item reappears (agents/ITEMS.md "Placed items" → "Respawn"), part of the item form (was
	 * the separate RespawnEditor of the read-only catalog page): never (a one-time find; no `respawn` key) or a default
	 * interval with the range a placement may choose (min … max). Units: every visit (reload), minutes, hours, days.
	 */
	let { value, onchange }: { value: ItemRespawn | null | undefined; onchange: (next: ItemRespawn | null) => void } = $props();

	type Kind = 'visit' | RespawnUnit;
	interface Part {
		kind: Kind;
		n: number;
	}

	const toPart = (i: RespawnInterval | undefined, fallback: Part): Part => {
		if (!i) return fallback;
		if (i === 'visit') return { kind: 'visit', n: 1 };
		const { unit, n } = intervalParts(i);
		return { kind: unit as RespawnUnit, n };
	};
	const toInterval = (p: Part): RespawnInterval => (p.kind === 'visit' ? 'visit' : ({ [p.kind]: Math.max(1, Math.floor(p.n || 1)) } as RespawnInterval));

	const parts = $derived({
		default: toPart(value?.default, { kind: 'hours', n: 6 }),
		min: toPart(value?.min ?? value?.default, { kind: 'hours', n: 1 }),
		max: toPart(value?.max, { kind: 'days', n: 7 })
	});
	const problems = $derived(itemRespawnProblems(value));

	function set(key: 'default' | 'min' | 'max', part: Part) {
		const next = { ...parts, [key]: part };
		onchange({ default: toInterval(next.default), min: toInterval(next.min), max: toInterval(next.max) });
	}
	function toggle(can: boolean) {
		onchange(can ? { default: { hours: 6 }, min: { hours: 1 }, max: { days: 7 } } : null);
	}
</script>

{#snippet part(key: 'default' | 'min' | 'max', title: string)}
	<label class="part">
		<span class="sc-caption">{title}</span>
		<span class="inputs">
			<select value={parts[key].kind} onchange={(e) => set(key, { kind: e.currentTarget.value as Kind, n: parts[key].n })} data-testid="respawn-{key}-unit">
				<option value="visit">{m['catalog.respawn_visit']()}</option>
				{#each RESPAWN_UNITS as u (u)}<option value={u}>{m[`catalog.respawn_unit_${u}`]()}</option>{/each}
			</select>
			{#if parts[key].kind !== 'visit'}
				<input type="number" min="1" step="1" value={parts[key].n} onchange={(e) => set(key, { kind: parts[key].kind, n: Number(e.currentTarget.value) })} aria-label="{title} n" data-testid="respawn-{key}-n" />
			{/if}
		</span>
	</label>
{/snippet}

<fieldset class="respawn" data-testid="respawn-field">
	<legend>{m['catalog.respawn']()}</legend>
	<label class="check"><input type="checkbox" checked={!!value} onchange={(e) => toggle(e.currentTarget.checked)} data-testid="respawn-can" /> {m['catalog.respawn_can']()}</label>
	{#if value}
		{@render part('default', m['catalog.respawn_default']())}
		{@render part('min', m['catalog.respawn_min']())}
		{@render part('max', m['catalog.respawn_max']())}
		{#each problems as p (p)}<p class="error">{p}</p>{/each}
	{/if}
	<p class="sc-caption muted">{m['catalog.respawn_hint']()}</p>
</fieldset>

<style>
	.respawn {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.part {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
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
	.error {
		margin: 0;
		color: var(--state-error);
	}
	p {
		margin: 0;
	}
</style>
