<script>
	// The Export step's panel: the chapter's handheld outputs (owner 2026-10-19: "a function that renders all shots made this
	// way as videos", "Also als Download zusätzlich zum Rendern als Video") – every shot with the handheld camera on and an
	// active take, rendered along its trimmed, smoothed track as control videos (the passes, 24 / 25 / 30 fps, 1× / 2×,
	// + WebM), or downloaded as one zip (GLB + JSON each). The scripts (Check, All frames, Reference packs, Chapter draft)
	// are the top bar's Chapter menu. README "Handheld camera".
	import { api, ch, fail, find, notify, studio, toon } from '../studio.svelte.js';
	import { PropRow, Segmented } from '@scutoons/ui';
	import { endPreview } from '../handheld/handheld.svelte.js';
	import { downloadAll } from '../handheld/export.js';
	import { PASSES, renderAll } from '../handheld/render.js';

	let opts = $state({ passes: ['beauty', 'depth'], fps: '24', ss: '1', webm: false });
	let withScene = $state(false);
	let busy = $state('');
	let results = $state([]);

	async function each(label, fn) {
		busy = label;
		try { await fn(); } catch (e) { fail(e); } finally { busy = ''; }
	}
	async function all() {
		const takes = await api('GET', `/api/takes/all?toon=${toon()}&ch=${ch()}`);
		if (!Object.keys(takes).length) { notify('No shot has the handheld camera on with an active take yet.', 'warning'); return null; }
		return takes;
	}
	const zipAll = () => each('Packing …', async () => {
		const takes = await all();
		if (takes) notify(`${await downloadAll(studio.chapter, takes, find, withScene, (m) => (busy = m))} shot(s) in the zip.`, 'success');
	});
	const renderShots = () => each('Rendering …', async () => {
		const takes = await all();
		if (!takes) return;
		endPreview();
		results = await renderAll(studio.chapter, takes, find, { passes: [...opts.passes], fps: Number(opts.fps), ss: Number(opts.ss), webm: opts.webm }, (m) => (busy = m));
		notify(`Rendered ${results.filter((r) => r.files).length} of ${results.length} handheld shot(s).`, 'success');
	});
	const togglePass = (p) => (opts.passes = opts.passes.includes(p) ? opts.passes.filter((x) => x !== p) : [...opts.passes, p]);
</script>

<div class="panel">
	<fieldset>
		<legend>Handheld shots as videos</legend>
		<PropRow label="Passes" hint="The control videos for Runway / Luma / Wan VACE / ControlNet">
			<div class="sc-seg is-stretch" role="group" aria-label="Passes">
				{#each PASSES as p (p.id)}
					<button type="button" aria-pressed={opts.passes.includes(p.id)} title={p.title} onclick={() => togglePass(p.id)}>{p.label}</button>
				{/each}
			</div>
		</PropRow>
		<PropRow label="Frames">
			<Segmented label="fps" stretch options={['24', '25', '30'].map((f) => ({ value: f, label: `${f} fps` }))} value={opts.fps} onchange={(v) => (opts.fps = v)} />
			<Segmented label="Size" stretch options={[{ value: '1', label: '1×' }, { value: '2', label: '2×' }]} value={opts.ss} onchange={(v) => (opts.ss = v)} />
		</PropRow>
		<PropRow label="Also">
			<Segmented label="WebM" options={[{ value: 'webm', label: '+ WebM', title: 'A VP9 .webm next to each .mp4' }]} value={opts.webm ? 'webm' : null} onchange={() => (opts.webm = true)} onclear={() => (opts.webm = false)} />
		</PropRow>
		<div class="buttons">
			<button type="button" onclick={renderShots} disabled={!!busy || !opts.passes.length} title="Every shot with the handheld camera on and an active take (storyboards/_out/<chapter>/handheld/)">Render all handheld shots</button>
		</div>
		{#each results as r (r.shot)}
			<p class="hint">{r.shot}: {#if r.files}{#each r.files as f, i (f)}{i ? ' · ' : ''}<a href={`/out/${f.replace(/^storyboards\/_out\//, '')}`} target="_blank" rel="noreferrer">{f.split('/').pop()}</a>{/each}{:else}{r.error}{/if}</p>
		{/each}
	</fieldset>
	<fieldset>
		<legend>Handheld cameras as a download</legend>
		<PropRow label="Scene" hint="With the blocked set + figures in each GLB">
			<Segmented label="With the scene" options={[{ value: 'scene', label: 'With the scene' }]} value={withScene ? 'scene' : null} onchange={() => (withScene = true)} onclear={() => (withScene = false)} />
		</PropRow>
		<div class="buttons">
			<button type="button" class="secondary" onclick={zipAll} disabled={!!busy} title="GLB + JSON of every handheld shot's active take + a README">Download all (zip)</button>
		</div>
	</fieldset>
	{#if busy}<p class="hint" role="status">{busy}</p>{/if}
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-15);
		padding: var(--space-075) var(--space-1) var(--space-15);
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
	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
</style>
