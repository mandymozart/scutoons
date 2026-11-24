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
	import { Button, Select, Toggle, type SelectOption } from '@scutoons/ui/editor';
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
	const instrumentOptions = $derived<SelectOption[]>(instruments.map((i) => ({ value: i.id, label: `${label(i.name, i.id)}${i.kind ? ` · ${i.kind}` : ''}` })));
	/** a track's instruments: the catalog's, and the one it names when the catalog lacks it */
	const instrumentsFor = (id: string): SelectOption[] => (instruments.some((i) => i.id === id) ? instrumentOptions : [{ value: id, label: id }, ...instrumentOptions]);
	const presetOptions = (instrument: string, none: string): SelectOption[] => [{ value: '', label: none }, ...presetsOf(instrument, presets).map((p) => ({ value: p.id, label: p.id }))];
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
		<Button onclick={oncreate} data-testid="assign-create">{m['designer.assign_create']()}</Button>
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
								<Select value={t.instrument} options={instrumentsFor(t.instrument)} label={m['designer.assign_instrument']()} onchange={(v) => onchange(setTrack(rig, t.id, { instrument: v }))} />
							</td>
							<td>
								<span class="cell">
									<Select value={t.preset ?? ''} options={presetOptions(t.instrument, m['designer.assign_bare']())} label={m['designer.assign_preset']()} disabled={!!presetOwner(t.id)} title={why(presetOwner(t.id))} onchange={(v) => onchange(setTrack(rig, t.id, { preset: v }))} data-testid="assign-track-preset" />
									{@render codeMark(presetOwner(t.id))}
								</span>
							</td>
							<td><input type="number" step="0.05" min="0" value={t.gain ?? 1} disabled={!!gainOwner(t.id)} title={why(gainOwner(t.id))} onchange={(e) => onchange(setTrack(rig, t.id, { gain: num(e.currentTarget.value, 1) }))} aria-label={m['designer.assign_gain']()} />{@render codeMark(gainOwner(t.id))}</td>
							<td><input type="number" step="0.1" min="-1" max="1" value={t.pan ?? 0} onchange={(e) => onchange(setTrack(rig, t.id, { pan: num(e.currentTarget.value, 0) }))} aria-label={m['designer.assign_pan']()} /></td>
							<td><Toggle on={t.mute ?? false} label={m['designer.assign_mute']()} onchange={(on) => onchange(setTrack(rig, t.id, { mute: on }))} /></td>
							<td><CloseButton label={m['designer.assign_remove']({ id: t.id })} stroke={1} onclick={() => onchange(removeTrack(rig, t.id))} /></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<div class="add">
			<span>{m['designer.assign_add']()}</span>
			<Select bind:value={adding} options={[{ value: '', label: '…' }, ...instrumentOptions]} label={m['designer.assign_add']()} onchange={(v) => v && (onchange(addTrack(rig, v, presets)), (adding = ''))} data-testid="assign-add" />
		</div>

		{#if rig.track.length > 0 && rows.length > 0}
			<h3 title={m['designer.assign_moods_hint']()}>{m['designer.assign_moods']()}</h3>
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
									<Select value={moodPreset(rig, mood, t.id) ?? ''} options={presetOptions(t.instrument, m['designer.assign_own']())} label="{mood} · {t.id}" disabled={!!presetOwner(t.id)} title={why(presetOwner(t.id))} onchange={(v) => onchange(setMoodPreset(rig, mood, t.id, v || null))} />
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}

		{#if owned.length > 0}
			<h3 title={m['designer.assign_owned_hint']({ module: face?.policy?.module ?? '' })}>{m['designer.assign_owned']()}</h3>
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
		color: var(--tool-text);
	}
	p {
		margin: 0 0 var(--space-05);
	}
	.muted {
		color: var(--tool-muted);
	}
	.status.missing,
	.problem {
		color: var(--tool-stop);
	}
	/* a section's head: the label type, muted (deck-kit's) */
	h3 {
		margin: var(--space-1) 0 var(--space-025);
		font-size: var(--type-label-size);
		font-weight: inherit;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--tool-muted);
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		padding: var(--space-025) var(--space-05) var(--space-025) 0;
		text-align: left;
		vertical-align: middle;
		font-weight: inherit;
		border-bottom: 1px solid var(--tool-line);
	}
	th {
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
	}
	.cell {
		display: flex;
		align-items: center;
		gap: var(--space-025);
	}
	input[type='number'] {
		width: 5.5rem;
		max-width: 100%;
		font-family: var(--font-mono);
		font-variant-numeric: tabular-nums;
	}
	input:disabled {
		opacity: 0.6;
	}
	.code {
		margin-left: var(--space-025);
		font-size: var(--type-label-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--tool-muted);
	}
	.add {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-top: var(--space-05);
	}
</style>
