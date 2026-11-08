<script lang="ts">
	// The Assignment tab (#283, RIG-EDITING.md §2 tab 4): the declarative rig of one character as a form – its tracks (an
	// instrument, its default preset, the mixer strip) and the preset each track plays per mood (the rig's mood variants:
	// the core replays them on every mood, #368). Every change goes up as a new document (`onchange`); the app saves it as a
	// draft, plays it on the deck and says what the core answered (`problem`). Whether the rig plays for the decks is the
	// core's (`coverage`, space-web `rig_coverage`). Not here yet: personas, routes into the being's look, the sticks, scene
	// and world variables (#283 Open). A code rig (#314, #412) drives the same document: what its policy owns – the core's
	// `face` says, no rule here – is read-only with the reason, and the inputs the form has no field for are listed.
	import { addTrack, faceTrack, moodPreset, moodRows, ownedBy, ownedInputs, presetsOf, removeTrack, setMoodPreset, setTrack, type CatalogInstrument, type CatalogPreset, type RigCoverage, type RigDoc, type RigDriven, type RigFace } from '../assignment.ts';
	import { CloseButton } from '@scutoons/ui';
	import { designerLocale } from '../setup.ts';
	import { m } from '../messages.ts';

	let { rig, instruments, presets, moods = [], coverage = null, problem = '', face = null, onchange, oncreate }: {
		/** the rig document (the draft as it stands); null = the character has none yet */
		rig: RigDoc | null;
		instruments: CatalogInstrument[];
		presets: CatalogPreset[];
		/** the moods offered beside the rig's own (the character's portraits) */
		moods?: string[];
		/** this character's default rig as the core sees the catalog */
		coverage?: RigCoverage | null;
		/** the core's answer when the deck played the draft ('' = it plays) */
		problem?: string;
		/** the draft as the core describes it: who owns what of a code rig */
		face?: RigFace | null;
		onchange: (rig: RigDoc) => void;
		oncreate: () => void;
	} = $props();

	const label = (name: Record<string, string> | undefined, id: string) => name?.[designerLocale()] ?? name?.en ?? id;
	const rows = $derived(rig ? moodRows(rig, moods) : []);
	let adding = $state('');
	const statusText = $derived(!coverage ? '' : coverage.default === 'plays' ? m['designer.assign_plays']() : coverage.default === 'drafted' ? m['designer.assign_drafted']() : m['designer.assign_missing']());
	const num = (v: string, fallback: number) => (Number.isFinite(Number.parseFloat(v)) ? Number.parseFloat(v) : fallback);
	const owned = $derived(ownedInputs(face));
	const why = (by: RigDriven | null) => (by ? m['designer.assign_driven']({ output: by.output, mode: by.mode }) : undefined);
	const presetOwner = (track: string) => ownedBy(faceTrack(face, track)?.preset_driven);
	const gainOwner = (track: string) => ownedBy(faceTrack(face, track)?.gain_driven);
</script>

{#snippet codeMark(by: RigDriven | null)}
	{#if by}<span class="code" title={why(by)} data-testid="assign-driven">{m['designer.assign_code']()}</span>{/if}
{/snippet}

<div class="assign" data-testid="designer-assignment">
	{#if coverage}
		<p class="status" class:missing={coverage.missing} data-testid="assign-status">{coverage.subject} · {statusText}</p>
		{#if coverage.missing}<p class="muted" data-testid="assign-fallback">{m['designer.assign_fallback']()}</p>{/if}
	{/if}
	{#if problem}<p class="problem" role="status" data-testid="assign-problem">{problem}</p>{/if}

	{#if !rig}
		<p class="muted">{m['designer.assign_none']()}</p>
		<button type="button" onclick={oncreate} data-testid="assign-create">{m['designer.assign_create']()}</button>
	{:else}
		<h3>{m['designer.assign_tracks']()}</h3>
		{#if rig.track.length > 0}
			<table>
				<thead>
					<tr><th>{m['designer.assign_track']()}</th><th>{m['designer.assign_instrument']()}</th><th>{m['designer.assign_preset']()}</th><th>{m['designer.assign_gain']()}</th><th>{m['designer.assign_pan']()}</th><th>{m['designer.assign_mute']()}</th><th></th></tr>
				</thead>
				<tbody>
					{#each rig.track as t (t.id)}
						<tr data-testid="assign-track">
							<td>{t.id}</td>
							<td>
								<select value={t.instrument} onchange={(e) => onchange(setTrack(rig, t.id, { instrument: e.currentTarget.value }))} aria-label={m['designer.assign_instrument']()}>
									{#if !instruments.some((i) => i.id === t.instrument)}<option value={t.instrument}>{t.instrument}</option>{/if}
									{#each instruments as i (i.id)}<option value={i.id}>{label(i.name, i.id)}{i.kind ? ` · ${i.kind}` : ''}</option>{/each}
								</select>
							</td>
							<td>
								<select value={t.preset ?? ''} disabled={!!presetOwner(t.id)} title={why(presetOwner(t.id))} onchange={(e) => onchange(setTrack(rig, t.id, { preset: e.currentTarget.value }))} aria-label={m['designer.assign_preset']()} data-testid="assign-track-preset">
									<option value="">{m['designer.assign_bare']()}</option>
									{#each presetsOf(t.instrument, presets) as p (p.id)}<option value={p.id}>{p.id}</option>{/each}
								</select>
								{@render codeMark(presetOwner(t.id))}
							</td>
							<td><input type="number" step="0.05" min="0" value={t.gain ?? 1} disabled={!!gainOwner(t.id)} title={why(gainOwner(t.id))} onchange={(e) => onchange(setTrack(rig, t.id, { gain: num(e.currentTarget.value, 1) }))} aria-label={m['designer.assign_gain']()} />{@render codeMark(gainOwner(t.id))}</td>
							<td><input type="number" step="0.1" min="-1" max="1" value={t.pan ?? 0} onchange={(e) => onchange(setTrack(rig, t.id, { pan: num(e.currentTarget.value, 0) }))} aria-label={m['designer.assign_pan']()} /></td>
							<td><input type="checkbox" checked={t.mute ?? false} onchange={(e) => onchange(setTrack(rig, t.id, { mute: e.currentTarget.checked }))} aria-label={m['designer.assign_mute']()} /></td>
							<td><CloseButton label={m['designer.assign_remove']({ id: t.id })} stroke={1} onclick={() => onchange(removeTrack(rig, t.id))} /></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<label class="add">
			<span>{m['designer.assign_add']()}</span>
			<select bind:value={adding} onchange={() => adding && (onchange(addTrack(rig, adding, presets)), (adding = ''))} data-testid="assign-add">
				<option value="">…</option>
				{#each instruments as i (i.id)}<option value={i.id}>{label(i.name, i.id)}{i.kind ? ` · ${i.kind}` : ''}</option>{/each}
			</select>
		</label>

		{#if rig.track.length > 0 && rows.length > 0}
			<h3>{m['designer.assign_moods']()}</h3>
			<p class="muted">{m['designer.assign_moods_hint']()}</p>
			<table>
				<thead>
					<tr><th>{m['designer.assign_mood']()}</th>{#each rig.track as t (t.id)}<th>{t.id} {@render codeMark(presetOwner(t.id))}</th>{/each}</tr>
				</thead>
				<tbody>
					{#each rows as mood (mood)}
						<tr data-testid="assign-mood">
							<td>{mood}</td>
							{#each rig.track as t (t.id)}
								<td>
									<select value={moodPreset(rig, mood, t.id) ?? ''} disabled={!!presetOwner(t.id)} title={why(presetOwner(t.id))} onchange={(e) => onchange(setMoodPreset(rig, mood, t.id, e.currentTarget.value || null))} aria-label="{mood} · {t.id}">
										<option value="">{m['designer.assign_own']()}</option>
										{#each presetsOf(t.instrument, presets) as p (p.id)}<option value={p.id}>{p.id}</option>{/each}
									</select>
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}

		{#if owned.length > 0}
			<h3>{m['designer.assign_owned']()}</h3>
			<p class="muted">{m['designer.assign_owned_hint']({ module: face?.policy?.module ?? '' })}</p>
			<table data-testid="assign-owned">
				<thead>
					<tr><th>{m['designer.assign_input']()}</th><th>{m['designer.assign_output']()}</th><th>{m['designer.assign_mode']()}</th></tr>
				</thead>
				<tbody>
					{#each owned as row (row.path)}
						<tr title={why(row.by)} data-testid="assign-owned-row"><td>{row.path}</td><td>{row.by.output}</td><td>{row.by.mode}</td></tr>
					{/each}
				</tbody>
			</table>
		{/if}
	{/if}
</div>

<style>
	.assign {
		height: 100%;
		overflow: auto;
		padding: var(--space-1);
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
	p {
		margin: 0 0 var(--space-05);
	}
	.muted {
		color: var(--text-muted);
	}
	.status.missing,
	.problem {
		color: var(--state-error);
	}
	h3 {
		margin: var(--space-1) 0 var(--space-05);
		font-size: 0.75rem;
		font-weight: inherit;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		color: var(--text-muted);
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		padding: var(--space-025) var(--space-05) var(--space-025) 0;
		text-align: left;
		font-weight: inherit;
		border-bottom: var(--border-width-hairline) solid color-mix(in srgb, var(--ink) 20%, transparent);
	}
	th {
		color: var(--text-muted);
		text-transform: uppercase;
	}
	select,
	input[type='number'] {
		max-width: 100%;
		font: inherit;
	}
	input[type='number'] {
		width: 4rem;
	}
	select:disabled,
	input:disabled {
		opacity: 0.6;
	}
	.code {
		margin-left: var(--space-025);
		color: var(--text-muted);
		text-transform: uppercase;
	}
	.add {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-top: var(--space-05);
	}
</style>
