<script>
	// The handheld camera of a shot (owner 2026-10-19: "In the storyboard module I'd enable it per shot … on the PC the
	// camera of that shot … moves along with the phone and records the camera movement. Afterwards I only need to trim
	// … and then a function that renders all shots made this way as videos"; "Also als Download"). The switch per shot,
	// the phone (PhonePairing), the takes (one active, its trim + smoothing), the downloads, and for the whole chapter:
	// render every handheld shot as videos + one zip. README "Handheld camera", research/HANDHELD-CAMERA.md.
	import { confirmDialog, Icon, IconButton } from '@scutoons/ui';
	import { api, ch, find, studio, toon } from '../studio.svelte.js';
	import { activeTake, endPreview, hh, loadDoc, pause, play, preview, resume, saveDoc, sendSpec, setOn } from '../handheld/handheld.svelte.js';
	import { downloadAll, downloadTake } from '../handheld/export.js';
	import { PASSES, renderAll } from '../handheld/render.js';
	import PhonePairing from './PhonePairing.svelte';
	import TakeTimeline from './TakeTimeline.svelte';

	const id = $derived(studio.sel?.id);
	$effect(() => { if (id && hh.docShot !== id) { endPreview(); loadDoc(id).then(resume).catch((e) => (studio.message = e.message)); } });
	// the phone shows the edited shot as it is (unsaved edits too)
	$effect(() => { if (studio.sel) { void JSON.stringify($state.snapshot(studio.sel.shot)); sendSpec(); } });
	const take = $derived(hh.doc ? activeTake() : null);
	const duration = $derived(take?.samples.length ? take.samples[take.samples.length - 1].t : 0);
	let withScene = $state(false);
	let opts = $state({ passes: ['beauty', 'depth'], fps: 24, ss: 1, webm: false });
	let busy = $state('');
	let results = $state([]);

	function pick(tid) {
		endPreview();
		hh.doc.active = tid;
		saveDoc();
	}
	async function remove(t) {
		if (!(await confirmDialog({ title: `Delete take ${t.id}?`, message: `${t.samples.length} samples, ${(t.out - t.in).toFixed(1)} s kept.`, confirmLabel: 'Delete' }))) return;
		hh.doc.takes = hh.doc.takes.filter((x) => x.id !== t.id);
		if (hh.doc.active === t.id) hh.doc.active = hh.doc.takes.at(-1)?.id ?? null;
		endPreview();
		saveDoc();
	}
	function trim(tin, tout) {
		take.in = +tin.toFixed(3);
		take.out = +tout.toFixed(3);
		saveDoc(true);
	}
	async function each(label, fn) {
		busy = label;
		try { await fn(); } catch (e) { studio.message = e.message; } finally { busy = ''; }
	}
	const one = (kind) => each('Exporting …', () => downloadTake(find(id), id, $state.snapshot(take), kind, withScene));
	async function all() {
		const takes = await api('GET', `/api/takes/all?toon=${toon()}&ch=${ch()}`);
		if (!Object.keys(takes).length) { studio.message = 'No shot has the handheld camera on with an active take yet.'; return null; }
		return takes;
	}
	const zipAll = () => each('Packing …', async () => {
		const takes = await all();
		if (takes) studio.message = `${await downloadAll(studio.chapter, takes, find, withScene, (m) => (busy = m))} shot(s) in the zip.`;
	});
	const renderShots = () => each('Rendering …', async () => {
		const takes = await all();
		if (!takes) return;
		endPreview();
		results = await renderAll(studio.chapter, takes, find, $state.snapshot(opts), (m) => (busy = m));
		studio.message = `Rendered ${results.filter((r) => r.files).length} of ${results.length} handheld shot(s).`;
	});
	const togglePass = (p, on) => (opts.passes = on ? [...opts.passes, p] : opts.passes.filter((x) => x !== p));
</script>

<div class="panel">
	{#if id && hh.doc}
		<label class="check"><input type="checkbox" checked={hh.doc.on} onchange={(e) => setOn(e.currentTarget.checked)} /> Handheld camera for {id}</label>
		{#if hh.doc.on}
			<fieldset>
				<legend>Phone</legend>
				<PhonePairing />
			</fieldset>
			{#if hh.doc.takes.length}
			<fieldset>
				<legend>Takes</legend>
				<ul class="takes">
					{#each hh.doc.takes as t (t.id)}
						<li>
							<label class="check"><input type="radio" name="take" checked={hh.doc.active === t.id} onchange={() => pick(t.id)} /> {t.id} · {(t.out - t.in).toFixed(1)} s of {t.samples.at(-1)?.t.toFixed(1)} s · {t.created.slice(11, 16)}</label>
							<IconButton label={`Delete take ${t.id}`} class="sc-round" onclick={() => remove(t)}><Icon name="trash-2" /></IconButton>
						</li>
					{/each}
				</ul>
				{#if take}
					<TakeTimeline {duration} tin={take.in} tout={take.out} playhead={hh.playhead} playing={hh.playing} ontrim={trim} onscrub={(t) => { pause(); preview(t); }} onplay={play} onpause={pause} />
					<label class="range">Smoothing of {take.id} <span>{(take.smooth ?? 0).toFixed(2)}</span>
						<input type="range" min="0" max="1" step="0.05" value={take.smooth ?? 0} oninput={(e) => { take.smooth = Number(e.currentTarget.value); preview(hh.playhead); saveDoc(true); }} />
					</label>
					<div class="buttons">
						<button type="button" class="secondary" onclick={endPreview} title="The view's camera back to the shot camera">Shot camera</button>
						<button type="button" class="secondary" onclick={() => one('json')} disabled={!!busy} title="The samples (t, position, quaternion, fov) for other tools">JSON</button>
						<button type="button" class="secondary" onclick={() => one('glb')} disabled={!!busy} title="An animated camera for Blender (File → Import → glTF 2.0) / Unreal (glTF importer → Sequencer)">Camera as glTF</button>
					</div>
				{/if}
			</fieldset>
			{/if}
		{/if}
	{/if}

	<fieldset>
		<legend>All handheld shots of the chapter</legend>
		<div class="passes" role="group" aria-label="Passes">
			{#each PASSES as p (p.id)}
				<label class="check" title={p.title}><input type="checkbox" checked={opts.passes.includes(p.id)} onchange={(e) => togglePass(p.id, e.currentTarget.checked)} /> {p.label}</label>
			{/each}
		</div>
		<div class="sc-fields" data-cols="3" data-density="panel">
			<label class="range">fps <select bind:value={opts.fps}>{#each [24, 25, 30] as f (f)}<option value={f}>{f}</option>{/each}</select></label>
			<label class="range">Size <select bind:value={opts.ss}><option value={1}>1×</option><option value={2}>2×</option></select></label>
			<label class="check"><input type="checkbox" bind:checked={opts.webm} /> + WebM</label>
		</div>
		<label class="check"><input type="checkbox" bind:checked={withScene} /> Downloads with the scene (set + figures)</label>
		<div class="buttons">
			<button type="button" class="secondary" onclick={zipAll} disabled={!!busy} title="GLB + JSON of every handheld shot's active take">Download all (zip)</button>
			<button type="button" onclick={renderShots} disabled={!!busy || !opts.passes.length} title="Every shot with the handheld camera on and an active take, as videos (storyboards/_out/<chapter>/handheld/)">Render all handheld shots</button>
		</div>
		{#if busy}<p class="hint" role="status">{busy}</p>{/if}
		{#each results as r (r.shot)}
			<p class="hint">{r.shot}: {#if r.files}{#each r.files as f, i (f)}{i ? ' · ' : ''}<a href={`/out/${f.replace(/^storyboards\/_out\//, '')}`} target="_blank" rel="noreferrer">{f.split('/').pop()}</a>{/each}{:else}{r.error}{/if}</p>
		{/each}
	</fieldset>
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
		padding: var(--space-1);
		align-content: start;
	}
	fieldset {
		border: 0;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-05);
		min-width: 0;
	}
	legend {
		padding: 0;
		margin-bottom: var(--space-05);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		font-size: var(--type-caption-size);
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
	}
	.takes {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-025);
	}
	.takes li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
	}
	.range {
		display: grid;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
	}
	.passes {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
</style>
