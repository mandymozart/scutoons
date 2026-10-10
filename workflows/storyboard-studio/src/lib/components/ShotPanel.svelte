<script>
	// The shot's fields (workflows/scenes/storyboard.md): what it shows, format + camera, light, figures – each change
	// re-renders the 3D view. Field groups without frames (legend = label), one control height (tools.css). Actions:
	// save (+ the frame), keep as a variant, revert, the review (→ approve.mjs) and a rule for taste.md.
	import { askDialog, Field, IconButton, Icon } from '@scutoons/ui';
	import { api, ch, revert, save, saveVariant, studio, toon, touched, VOCAB } from '../studio.svelte.js';
	import { framePng } from '../frames.js';

	const s = $derived(studio.sel?.shot);
	const castRefs = $derived((studio.sel?.scene.cast ?? []).map((c) => c.ref));
	const sceneLight = $derived(studio.sel?.scene.light ?? {});
	let busy = $state('');

	function set(obj, key, value) {
		if (value === '' || value == null) delete obj[key];
		else obj[key] = value;
		touched();
	}
	function ensure(key, init) {
		s[key] ??= init;
		return s[key];
	}
	async function doSave() {
		busy = 'Saving …';
		try { await save(await framePng()); } finally { busy = ''; }
	}
	async function doVariant() {
		const note = await askDialog({ title: 'Keep as a variant', label: 'What is different?', confirmLabel: 'Keep', skipLabel: 'Keep without note' });
		if (note === null) return;
		busy = 'Keeping …';
		try { await saveVariant(note, await framePng()); } finally { busy = ''; }
	}
	async function review(status) {
		const note = await askDialog({ title: status === 'ok' ? 'Approve the shot' : 'Ask for a change', label: status === 'ok' ? 'Note (optional)' : 'What should change?', confirmLabel: status === 'ok' ? 'Approve' : 'Ask', skipLabel: status === 'ok' ? 'Approve without note' : undefined });
		if (note === null || (status === 'change' && !note)) return;
		if (studio.dirty) await doSave();
		const r = await api('POST', '/api/review', { toon: toon(), ch: ch(), id: s.id, status, note });
		studio.message = r.out;
	}
	async function rule() {
		const text = await askDialog({ title: 'A rule for the storyboards', message: 'Goes into workflows/scenes/storyboard-taste.md – the drafting agents read it.', label: 'The rule, in your words', confirmLabel: 'Add' });
		if (!text) return;
		await api('POST', '/api/taste', { rule: text, quote: '' });
		studio.message = 'Added to storyboard-taste.md.';
	}
	const num = (v) => (v === '' ? null : Number(v));
</script>

{#if s}
	<div class="panel">
		<h3>{s.id}{studio.dirty ? ' · unsaved' : ''}</h3>

		<fieldset>
			<legend>What it shows</legend>
			<div class="sc-fields" data-density="panel">
				<Field label="Action (de)">{#snippet children({ id })}<textarea {id} rows="4" value={s.action?.de ?? ''} oninput={(e) => { s.action = { ...(s.action ?? {}), de: e.currentTarget.value }; touched(); }}></textarea>{/snippet}</Field>
				<Field label="Image (en, for prompts)">{#snippet children({ id })}<textarea {id} rows="3" value={s.prompt?.en ?? ''} oninput={(e) => set(s, 'prompt', e.currentTarget.value ? { en: e.currentTarget.value } : null)}></textarea>{/snippet}</Field>
			</div>
			<div class="sc-fields" data-cols="2" data-density="panel">
				<Field label="Perspective">{#snippet children({ id })}<select {id} value={s.view ?? ''} onchange={(e) => set(s, 'view', e.currentTarget.value)}>{#each ['', ...VOCAB.view] as o}<option value={o}>{o || '–'}</option>{/each}</select>{/snippet}</Field>
				<Field label="Source">{#snippet children({ id })}<select {id} value={s.source?.want ?? ''} onchange={(e) => { ensure('source', { type: 'schematic' }).want = e.currentTarget.value; touched(); }}>{#each VOCAB.want as o}<option value={o}>{o}</option>{/each}</select>{/snippet}</Field>
			</div>
			<div class="sc-fields" data-density="panel">
				<Field label="Note">{#snippet children({ id })}<textarea {id} rows="2" value={s.note ?? ''} oninput={(e) => set(s, 'note', e.currentTarget.value)}></textarea>{/snippet}</Field>
			</div>
		</fieldset>

		<fieldset>
			<legend>Format + camera</legend>
			<div class="sc-fields" data-cols="2" data-density="panel">
				{#each [['format', 'Format'], ['aspect', 'Aspect'], ['projection', 'Projection'], ['move', 'Scroll move']] as [k, label] (k)}
					<Field {label}>{#snippet children({ id })}<select {id} value={s[k] ?? ''} onchange={(e) => set(s, k, e.currentTarget.value)}>{#each VOCAB[k] as o}<option value={o}>{o}</option>{/each}</select>{/snippet}</Field>
				{/each}
				<Field label="Size">{#snippet children({ id })}<select {id} value={s.shot?.size ?? ''} onchange={(e) => { ensure('shot', {}).size = e.currentTarget.value; touched(); }}>{#each VOCAB.size as o}<option value={o}>{o}</option>{/each}</select>{/snippet}</Field>
				<Field label="Angle">{#snippet children({ id })}<select {id} value={s.shot?.angle ?? ''} onchange={(e) => { ensure('shot', {}).angle = e.currentTarget.value; touched(); }}>{#each VOCAB.angle as o}<option value={o}>{o}</option>{/each}</select>{/snippet}</Field>
			</div>
			<p class="hint">{s.camera?.pos ? `Hand camera (3D view): at ${s.camera.pos.join(', ')} · lens ${s.camera.fov}°` : 'The camera comes from size + angle – move it in the 3D view to set it by hand.'}</p>
			{#if s.camera}<button type="button" class="secondary" onclick={() => { delete s.camera; touched(); }}>Back to the shot camera</button>{/if}
		</fieldset>

		<fieldset>
			<legend>Light <span class="hint">(scene: {sceneLight.key ?? '–'} · {sceneLight.height ?? '–'} · {sceneLight.source ?? '–'})</span></legend>
			<div class="sc-fields" data-cols="2" data-density="panel">
				{#each [['key', 'From'], ['height', 'Height'], ['source', 'Source'], ['shadow', 'Shadow'], ['accent', 'Accent']] as [k, label] (k)}
					<Field {label}>{#snippet children({ id })}<select {id} value={k === 'accent' ? s.light?.accent?.color ?? '' : s.light?.[k] ?? ''} onchange={(e) => { const L = ensure('light', {}); if (k === 'accent') set(L, 'accent', e.currentTarget.value ? { ...(L.accent ?? {}), color: e.currentTarget.value } : null); else set(L, k, e.currentTarget.value); }}>{#each VOCAB[k] as o}<option value={o}>{o || 'scene'}</option>{/each}</select>{/snippet}</Field>
				{/each}
				<Field label="Rim light">{#snippet children({ id })}<input {id} type="checkbox" checked={!!s.light?.rim} onchange={(e) => set(ensure('light', {}), 'rim', e.currentTarget.checked || null)} />{/snippet}</Field>
				<Field label="Azimuth °" hint="free direction: 0 = from the camera, 90 = right, 180 = behind">{#snippet children({ id })}<input {id} type="range" min="-180" max="180" value={s.light?.azimuth ?? -45} oninput={(e) => set(ensure('light', {}), 'azimuth', Number(e.currentTarget.value))} />{/snippet}</Field>
				<Field label="Softness" hint="0 = Sin City hard (solid black shadows) … 1 = soft (hatched shadows, a fill)">{#snippet children({ id })}<input {id} type="range" min="0" max="1" step="0.05" value={s.light?.soft ?? 0.5} oninput={(e) => set(ensure('light', {}), 'soft', Number(e.currentTarget.value))} />{/snippet}</Field>
				<Field label="Elevation °">{#snippet children({ id })}<input {id} type="range" min="-40" max="90" value={s.light?.elevation ?? 30} oninput={(e) => set(ensure('light', {}), 'elevation', Number(e.currentTarget.value))} />{/snippet}</Field>
			</div>
			{#if s.light?.azimuth != null || s.light?.elevation != null}<button type="button" class="secondary" onclick={() => { delete s.light.azimuth; delete s.light.elevation; touched(); }}>Free direction off</button>{/if}
			<div class="sc-fields" data-density="panel">
				<Field label="What the light does (de)">{#snippet children({ id })}<input {id} type="text" value={s.light?.note?.de ?? ''} oninput={(e) => set(ensure('light', {}), 'note', e.currentTarget.value ? { de: e.currentTarget.value } : null)} />{/snippet}</Field>
			</div>
		</fieldset>

		<fieldset>
			<legend>Figures</legend>
			{#each s.figures ?? [] as f, i (i)}
				<div class="figure" class:sc-selected={studio.figure === i}>
					<div class="sc-fields" data-cols="2" data-density="panel">
						<Field label="Cast">{#snippet children({ id })}<select {id} value={f.cast} onchange={(e) => set(f, 'cast', e.currentTarget.value)}>{#each castRefs as o}<option value={o}>{o}</option>{/each}</select>{/snippet}</Field>
						{#each [['persona', 'Persona'], ['pose', 'Pose'], ['facing', 'Facing'], ['framing', 'Framing'], ['mood', 'Mood']] as [k, label] (k)}
							<Field {label}>{#snippet children({ id })}<select {id} value={f[k] ?? ''} onchange={(e) => { set(f, k, e.currentTarget.value); if (k === 'facing') delete f.turn; }}>{#each (k === 'persona' || k === 'mood' ? VOCAB[k] : VOCAB[k]) as o}<option value={o}>{o || '–'}</option>{/each}</select>{/snippet}</Field>
						{/each}
						{#each [['x', 'x'], ['y', 'y'], ['h', 'Height (screens)']] as [k, label] (k)}
							<Field {label}>{#snippet children({ id })}<input {id} type="number" step="0.01" value={f[k] ?? ''} oninput={(e) => set(f, k, num(e.currentTarget.value))} />{/snippet}</Field>
						{/each}
					</div>
					<IconButton label="Remove the figure" class="sc-round" onclick={() => { s.figures.splice(i, 1); touched(); }}><Icon name="trash-2" /></IconButton>
				</div>
			{/each}
			<button type="button" class="secondary" onclick={() => { (s.figures ??= []).push({ cast: castRefs[0], pose: 'stand', facing: 'front', framing: 'full', x: 0.5, y: 0.6, h: 0.3 }); touched(); }}><Icon name="plus" /> Figure</button>
		</fieldset>

		<div class="actions">
			<button type="button" class="secondary" onclick={() => review('change')} title="An open change – shows as a warning until it is done">Ask for a change</button>
			<button type="button" class="secondary" onclick={() => review('ok')} title="Approve: the review + the dataset (approve.mjs)">Approve</button>
			<button type="button" class="secondary" onclick={rule} title="A rule for storyboard-taste.md">Rule</button>
			<button type="button" class="secondary" onclick={revert} disabled={!studio.dirty}>Revert</button>
			<button type="button" class="secondary" onclick={doVariant}>Keep as variant</button>
			<button type="button" onclick={doSave} disabled={!!busy}>{busy || 'Save + frame'}</button>
		</div>
	</div>
{/if}

<style>
	/* the dock gives the panel its height: it scrolls on its own, its rows keep their own height */
	.panel {
		padding: var(--space-1);
		display: grid;
		gap: var(--space-075);
		align-content: start;
		overflow-y: auto;
		box-sizing: border-box;
	}
	h3 {
		margin: 0;
		font-size: var(--type-body-size, 1rem);
		font-weight: var(--font-weight-regular);
	}
	fieldset {
		border: 0;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-05);
	}
	legend {
		padding: 0;
		margin-bottom: var(--space-05);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		font-size: var(--type-caption-size);
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		text-transform: none;
		letter-spacing: normal;
		opacity: 0.75;
	}
	.figure {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: start;
		gap: var(--space-05);
		padding: var(--space-05) 0;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	textarea {
		width: 100%;
		resize: vertical;
	}
	.actions {
		position: sticky;
		bottom: 0;
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
		padding: var(--space-05) 0;
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	/* the buttons share each row: fewer rows of the sticky bar over the fields */
	.actions button {
		flex: 1 1 auto;
	}
</style>
