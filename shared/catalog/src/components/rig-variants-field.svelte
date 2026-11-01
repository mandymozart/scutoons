<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { problemFor, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { presetsOf, renameKey, withVariantPreset, withVariantValue } from '../rigs/edit.ts';

	/**
	 * A rig's variants of one kind – `personas` or `moods` (research/INSTRUMENTS.md §3): per name another preset per
	 * track ("as the track" = none) and single values `<track>:<node>.<port>` over the defaults. A persona applies
	 * first, the mood after it. Problems the server or the validator name for a variant show under it.
	 */
	let {
		label,
		kind,
		variants,
		tracks,
		presets,
		paths,
		names = [],
		addLabel,
		problems,
		onchange
	}: {
		label: string;
		/** the word the problems use: "persona" / "mood" */
		kind: 'persona' | 'mood';
		variants: Record<string, Data>;
		tracks: Data[];
		presets: { id: string; data: Data }[];
		/** the `<track>:<node>.<port>` paths a value may set */
		paths: string[];
		/** suggested names (the moods registry) */
		names?: readonly string[];
		addLabel: string;
		problems: string[];
		onchange: (next: Record<string, Data>) => void;
	} = $props();

	const listId = $props.id();
	const entries = $derived(Object.entries(variants));
	const err = (name: string) => problemFor(problems, `${kind} ${name}`);
	const set = (name: string, next: Data) => onchange({ ...variants, [name]: next });
	const remove = (name: string) => onchange(Object.fromEntries(entries.filter(([k]) => k !== name)));
	function add() {
		const free = names.find((n) => !(n in variants));
		let name = free ?? 'new';
		for (let n = 2; name in variants; n++) name = `new${n}`;
		onchange({ ...variants, [name]: {} });
	}
	/** a value's path: the first free one */
	const freePath = (v: Data) => paths.find((p) => !(p in (v.values ?? {}))) ?? '';
	const number = (raw: string) => (raw.trim() === '' || Number.isNaN(Number(raw)) ? null : Number(raw));
</script>

<fieldset class="variants" data-testid="rig-{kind}s">
	<legend>{label}</legend>
	{#if names.length}<datalist id={listId}>{#each names as n (n)}<option value={n}></option>{/each}</datalist>{/if}
	{#each entries as [name, v] (name)}
		<div class="variant" data-testid="rig-{kind}-{name}">
			<div class="head">
				<input type="text" value={name} list={names.length ? listId : undefined} aria-label={m['catalog.rig_variant_name']()} onchange={(e) => onchange(renameKey(variants, name, e.currentTarget.value.trim()))} data-testid="rig-{kind}-name" />
				<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => remove(name)}><CloseIcon /></button>
			</div>
			{#each tracks as t (t.id)}
				<label class="row">
					<span class="sc-caption">{t.id}</span>
					<select value={v.presets?.[t.id] ?? ''} onchange={(e) => set(name, withVariantPreset(v, t.id, e.currentTarget.value))} data-testid="rig-{kind}-{name}-{t.id}">
						<option value="">{m['catalog.rig_variant_preset']()}</option>
						{#each presetsOf(presets, t.instrument) as p (p.id)}<option value={p.id}>{p.id}</option>{/each}
						{#if v.presets?.[t.id] && !presetsOf(presets, t.instrument).some((p) => p.id === v.presets[t.id])}<option value={v.presets[t.id]}>{v.presets[t.id]} (?)</option>{/if}
					</select>
				</label>
			{/each}
			{#each Object.entries(v.values ?? {}) as [path, value] (path)}
				<div class="value">
					<input type="text" list="{listId}-paths" value={path} aria-label={m['catalog.rig_variant_values']()} onchange={(e) => set(name, withVariantValue(withVariantValue(v, path, null), e.currentTarget.value.trim(), value as number))} />
					<input type="number" step="any" value={value} aria-label={path} onchange={(e) => set(name, withVariantValue(v, path, number(e.currentTarget.value)))} />
					<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => set(name, withVariantValue(v, path, null))}><CloseIcon /></button>
				</div>
			{/each}
			<span><Button size="sm" variant="secondary" disabled={!paths.length} onclick={() => set(name, withVariantValue(v, freePath(v), 0))}>{m['catalog.rig_add_value']()}</Button></span>
			{#if err(name)}<p class="sc-error">{err(name)}</p>{/if}
		</div>
	{/each}
	<datalist id="{listId}-paths">{#each paths as p (p)}<option value={p}></option>{/each}</datalist>
	<span><Button size="sm" variant="secondary" onclick={add} data-testid="rig-add-{kind}">{addLabel}</Button></span>
</fieldset>

<style>
	.variants {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-15);
	}
	.variant {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-05) 0 var(--space-05) var(--space-1);
		border-left: 2px solid var(--ink);
	}
	.head,
	.value {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-05);
		align-items: center;
	}
	.value {
		grid-template-columns: minmax(0, 1fr) 6rem auto;
	}
	.row {
		display: grid;
		grid-template-columns: 6rem minmax(0, 1fr);
		gap: var(--space-05);
		align-items: center;
	}
</style>
