<script lang="ts">
	import { MOODS } from '@scutoons/content';
	import { Button, CloseIcon } from '@scutoons/ui';
	import { problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { knobPaths } from '../rigs/validate.ts';
	import { lookupOf, newTrackId, presetsOf, routeEnds, sticksOf, trackPaths, withStick } from '../rigs/edit.ts';
	import CheckField from './check-field.svelte';
	import RigVariantsField from './rig-variants-field.svelte';
	import RightsField from './rights-field.svelte';

	/**
	 * A rig (research/INSTRUMENTS.md §3, deck-space rig/mod.rs): a character's or an object's tracks – each an
	 * instrument with its default preset and strip (gain, pan, mute) –, routes between tracks and to the being's look,
	 * persona and mood variants, the phone deck's sticks and the rights. The subject is the entry's id. Every change goes
	 * through `data = …` (one undo step each); the problems are the server's and the validator's, worded alike.
	 */
	let { data = $bindable(), id, lists, problems }: CatalogFormProps = $props();

	const instruments = $derived(lists.instruments ?? []);
	const presets = $derived(lists.presets ?? []);
	const I = $derived(lookupOf(instruments));
	const tracks = $derived<Data[]>(Array.isArray(data.track) ? data.track : []);
	const routes = $derived<Data[]>(Array.isArray(data.route) ? data.route : []);
	const paths = $derived(trackPaths(data, I));
	const ends = $derived(routeEnds(data, I));
	const knobs = $derived(knobPaths(data, I));
	const sticks = $derived(sticksOf(data));
	const err = (...words: string[]) => problemFor(problems, ...words);
	const number = (raw: string, fallback: number) => (raw.trim() === '' || Number.isNaN(Number(raw)) ? fallback : Number(raw));

	function setTrack(i: number, patch: Data) {
		const next = { ...tracks[i], ...patch };
		for (const k of ['preset'] as const) if (next[k] === '' || next[k] === null) delete next[k];
		data = { ...data, track: tracks.map((t, j) => (j === i ? next : t)) };
	}
	const addTrack = () => (data = { ...data, track: [...tracks, { id: newTrackId(data), instrument: instruments[0]?.id ?? '', gain: 1, pan: 0, mute: false }] });
	const removeTrack = (i: number) => (data = { ...data, track: tracks.filter((_, j) => j !== i) });
	function setRoute(i: number, patch: Data) {
		data = { ...data, route: routes.map((r, j) => (j === i ? { ...r, ...patch } : r)) };
	}
	const addRoute = () => (data = { ...data, route: [...routes, { from: ends.from[0] ?? '', to: ends.to[0] ?? '' }] });
	function removeRoute(i: number) {
		const { route: _old, ...rest } = data;
		const left = routes.filter((_, j) => j !== i);
		data = left.length ? { ...rest, route: left } : rest;
	}
	function setVariants(key: 'personas' | 'moods', next: Record<string, Data>) {
		const { [key]: _old, ...rest } = data;
		data = Object.keys(next).length ? { ...rest, [key]: next } : rest;
	}
	/** the problems no track, variant or stick shows at itself */
	const rest = $derived(
		problems.filter((p) => {
			const names = [...tracks.map((t) => `track ${t.id}`), ...tracks.map((t) => `'${t.id}'`), ...Object.keys(data.personas ?? {}).map((n) => `persona ${n}`), ...Object.keys(data.moods ?? {}).map((n) => `mood ${n}`), 'stick', 'route'];
			return !problemFor([p], ...names);
		})
	);
	const STICKS = [
		[0, () => m['catalog.rig_stick_left'](), 'x'],
		[1, () => m['catalog.rig_stick_left'](), 'y'],
		[2, () => m['catalog.rig_stick_right'](), 'x'],
		[3, () => m['catalog.rig_stick_right'](), 'y']
	] as const;
</script>

<div class="subject">
	<span class="sc-label">{m['catalog.rig_subject']()}</span>
	<span data-testid="rig-subject">{data.subject || id}</span>
	<span class="sc-caption muted">{m['catalog.rig_subject_hint']()}</span>
</div>
{#if rest.length}<ul class="sc-error problems" role="alert" data-testid="rig-problems">{#each rest as p (p)}<li>{p}</li>{/each}</ul>{/if}

<fieldset class="group">
	<legend>{m['catalog.rig_tracks']()}</legend>
	{#each tracks as t, i (i)}
		{@const options = presetsOf(presets, t.instrument)}
		<div class="track" data-testid="rig-track-{t.id}">
			<div class="line">
				<input type="text" value={t.id} aria-label={m['catalog.rig_track_id']()} placeholder={m['catalog.rig_track_id']()} onchange={(e) => setTrack(i, { id: e.currentTarget.value.trim() })} data-testid="rig-track-id" />
				<select value={t.instrument ?? ''} aria-label={m['catalog.instrument']()} onchange={(e) => setTrack(i, { instrument: e.currentTarget.value, preset: '' })} data-testid="rig-track-instrument">
					<option value="">{m['catalog.choose']()}</option>
					{#each instruments as inst (inst.id)}<option value={inst.id}>{text(inst.data?.name) || inst.id} · {inst.id}</option>{/each}
					{#if t.instrument && !instruments.some((x) => x.id === t.instrument)}<option value={t.instrument}>{t.instrument} (?)</option>{/if}
				</select>
				<select value={t.preset ?? ''} aria-label={m['catalog.rig_preset']()} onchange={(e) => setTrack(i, { preset: e.currentTarget.value })} data-testid="rig-track-preset">
					<option value="">{m['catalog.rig_preset_none']()}</option>
					{#each options as p (p.id)}<option value={p.id}>{p.id}</option>{/each}
					{#if t.preset && !options.some((p) => p.id === t.preset)}<option value={t.preset}>{t.preset} (?)</option>{/if}
				</select>
				<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => removeTrack(i)}><CloseIcon /></button>
			</div>
			<div class="strip">
				<label><span class="sc-caption">{m['catalog.rig_gain']()}</span><input type="number" step="0.05" min="0" value={t.gain ?? 1} onchange={(e) => setTrack(i, { gain: number(e.currentTarget.value, 1) })} data-testid="rig-track-gain" /></label>
				<label><span class="sc-caption">{m['catalog.rig_pan']()}</span><input type="number" step="0.1" min="-1" max="1" value={t.pan ?? 0} onchange={(e) => setTrack(i, { pan: number(e.currentTarget.value, 0) })} data-testid="rig-track-pan" /></label>
				<CheckField label={m['catalog.rig_mute']()} checked={t.mute === true} onchange={(on) => setTrack(i, { mute: on })} testid="rig-track-mute" />
			</div>
			{#if err(`track ${t.id}`, `'${t.id}'`, `track[${i}]`)}<p class="sc-error">{err(`track ${t.id}`, `'${t.id}'`, `track[${i}]`)}</p>{/if}
		</div>
	{/each}
	<span><Button size="sm" variant="secondary" onclick={addTrack} data-testid="rig-add-track">{m['catalog.rig_add_track']()}</Button></span>
</fieldset>

<RigVariantsField label={m['catalog.rig_personas']()} kind="persona" variants={data.personas ?? {}} {tracks} {presets} {paths} addLabel={m['catalog.rig_add_persona']()} {problems} onchange={(next) => setVariants('personas', next)} />
<RigVariantsField label={m['catalog.rig_moods']()} kind="mood" variants={data.moods ?? {}} {tracks} {presets} {paths} names={MOODS} addLabel={m['catalog.rig_add_mood']()} {problems} onchange={(next) => setVariants('moods', next)} />
<p class="sc-caption muted hint">{m['catalog.rig_variants_hint']()}</p>

<fieldset class="group">
	<legend>{m['catalog.rig_routes']()}</legend>
	{#each routes as r, i (i)}
		<div class="route" data-testid="rig-route">
			<input type="text" list="rig-route-from" value={r.from ?? ''} aria-label={m['catalog.rig_route_from']()} placeholder={m['catalog.rig_route_from']()} onchange={(e) => setRoute(i, { from: e.currentTarget.value.trim() })} />
			<input type="text" list="rig-route-to" value={r.to ?? ''} aria-label={m['catalog.rig_route_to']()} placeholder={m['catalog.rig_route_to']()} onchange={(e) => setRoute(i, { to: e.currentTarget.value.trim() })} />
			<input type="number" step="any" value={r.scale ?? 1} aria-label={m['catalog.rig_route_scale']()} title={m['catalog.rig_route_scale']()} onchange={(e) => setRoute(i, { scale: number(e.currentTarget.value, 1) })} />
			<input type="number" step="any" value={r.offset ?? 0} aria-label={m['catalog.rig_route_offset']()} title={m['catalog.rig_route_offset']()} onchange={(e) => setRoute(i, { offset: number(e.currentTarget.value, 0) })} />
			<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => removeRoute(i)}><CloseIcon /></button>
		</div>
	{/each}
	<datalist id="rig-route-from">{#each ends.from as p (p)}<option value={p}></option>{/each}</datalist>
	<datalist id="rig-route-to">{#each ends.to as p (p)}<option value={p}></option>{/each}</datalist>
	<span><Button size="sm" variant="secondary" disabled={!tracks.length} onclick={addRoute} data-testid="rig-add-route">{m['catalog.rig_add_route']()}</Button></span>
	<p class="sc-caption muted">{m['catalog.rig_route_hint']()}</p>
	{#if err('route')}<p class="sc-error">{err('route')}</p>{/if}
</fieldset>

<fieldset class="group">
	<legend>{m['catalog.rig_sticks']()}</legend>
	<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
		{#each STICKS as [index, side, axis] (index)}
			<label class="stick">
				<span class="sc-caption">{side()} · {axis}</span>
				<select value={sticks[index]} onchange={(e) => (data = withStick(data, index, e.currentTarget.value))} data-testid="rig-stick-{index}">
					<option value="">{m['catalog.rig_stick_none']()}</option>
					{#each knobs as k (k)}<option value={k}>{k}</option>{/each}
					{#if sticks[index] && !knobs.includes(sticks[index])}<option value={sticks[index]}>{sticks[index]} (?)</option>{/if}
				</select>
			</label>
		{/each}
	</div>
	<p class="sc-caption muted">{m['catalog.rig_sticks_hint']()}</p>
	{#if err('stick', 'sticks')}<p class="sc-error">{err('stick', 'sticks')}</p>{/if}
</fieldset>

<RightsField {data} hint={m['catalog.rights_hint']()} onchange={(next) => (data = next)} testid="rig-rights" />

<style>
	.subject {
		display: flex;
		flex-direction: column;
		gap: var(--space-025, 0.25rem);
		margin: 0 0 var(--space-15);
	}
	.muted {
		color: var(--text-muted);
	}
	.hint,
	.group p {
		margin: 0;
	}
	.problems {
		margin: 0 0 var(--space-15);
		padding-left: 1rem;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-15);
	}
	.track {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-05) 0 var(--space-05) var(--space-1);
		border-left: 2px solid var(--ink);
	}
	.line {
		display: grid;
		grid-template-columns: 7rem minmax(0, 1fr) minmax(0, 1fr) auto;
		gap: var(--space-05);
		align-items: center;
	}
	.strip {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		align-items: end;
	}
	.strip label {
		display: flex;
		flex-direction: column;
		gap: var(--space-025, 0.25rem);
		width: 6rem;
	}
	.route {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) 5rem 5rem auto;
		gap: var(--space-05);
		align-items: center;
	}
	.stick {
		display: flex;
		flex-direction: column;
		gap: var(--space-025, 0.25rem);
	}
	@media (max-width: 767px) {
		.line {
			grid-template-columns: minmax(0, 1fr);
		}
		.line .sc-round {
			justify-self: end;
		}
		.route {
			grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		}
	}
</style>
