<script>
	// The Export step's panel: the chapter's handheld outputs (owner 2026-10-19: "a function that renders all shots made this
	// way as videos", "Also als Download zusätzlich zum Rendern als Video") – every shot with the handheld camera on and an
	// active take, rendered along its trimmed, smoothed track as control videos (the passes, 24 / 25 / 30 fps, 1× / 2×,
	// + WebM), or downloaded as one zip (GLB + JSON each). The scripts (Check, All frames, Reference packs, Chapter draft)
	// are the top bar's Chapter menu. README "Handheld camera". The state and the jobs live in output.svelte.js, so a step
	// change during a render keeps its progress and links.
	import { PropRow, Segmented } from '@scutoons/ui';
	import { studio } from '../studio.svelte.js';
	import { PASSES } from '../handheld/render.js';
	import { out, renderShots, togglePass, zipAll } from '../output.svelte.js';

	const results = $derived(out.chapter === studio.chapter ? out.results : []);
</script>

<div class="panel">
	<!-- the rows sit in a plain grid, never a grid fieldset (GeneratePanel: Chromium leaves a PropRow in one at 0 px);
	     one choice a row, so no segment's word wraps at the panel's width (DESIGN "One control height") -->
	<fieldset>
		<legend>Handheld shots as videos</legend>
		<div class="rows">
			<PropRow label="Passes" hint="The control videos for Runway / Luma / Wan VACE / ControlNet">
				<div class="sc-seg is-stretch" role="group" aria-label="Passes">
					{#each PASSES as p (p.id)}
						<button type="button" aria-pressed={out.opts.passes.includes(p.id)} title={p.title} onclick={() => togglePass(p.id)}>{p.label}</button>
					{/each}
				</div>
			</PropRow>
			<PropRow label="fps" hint="Frames a second">
				<Segmented label="Frames a second" stretch options={['24', '25', '30'].map((f) => ({ value: f, label: f, title: `${f} frames a second` }))} value={out.opts.fps} onchange={(v) => (out.opts.fps = v)} />
			</PropRow>
			<PropRow label="Size" hint="The frame's size: 1× = the shot's format, 2× = twice as wide and high">
				<Segmented label="Size" stretch options={[{ value: '1', label: '1×', title: "The shot's format" }, { value: '2', label: '2×', title: 'Twice as wide and high' }]} value={out.opts.ss} onchange={(v) => (out.opts.ss = v)} />
			</PropRow>
			<PropRow label="Also">
				<Segmented label="WebM" options={[{ value: 'webm', label: '+ WebM', title: 'A VP9 .webm next to each .mp4' }]} value={out.opts.webm ? 'webm' : null} onchange={() => (out.opts.webm = true)} onclear={() => (out.opts.webm = false)} />
			</PropRow>
			<div class="buttons">
				<button type="button" onclick={renderShots} disabled={!!out.busy || !out.opts.passes.length} title="Every shot with the handheld camera on and an active take (storyboards/_out/<chapter>/handheld/)">Render all handheld shots</button>
			</div>
			{#each results as r (r.shot)}
				<p class="hint">{r.shot}: {#if r.files}{#each r.files as f, i (f)}{i ? ' · ' : ''}<a href={`/out/${f.replace(/^storyboards\/_out\//, '')}`} target="_blank" rel="noreferrer">{f.split('/').pop()}</a>{/each}{:else}{r.error}{/if}</p>
			{/each}
		</div>
	</fieldset>
	<fieldset>
		<legend>Handheld cameras as a download</legend>
		<div class="rows">
			<PropRow label="Scene" hint="With the blocked set + figures in each GLB">
				<Segmented label="With the scene" options={[{ value: 'scene', label: 'With the scene' }]} value={out.withScene ? 'scene' : null} onchange={() => (out.withScene = true)} onclear={() => (out.withScene = false)} />
			</PropRow>
			<div class="buttons">
				<button type="button" class="secondary" onclick={zipAll} disabled={!!out.busy} title="GLB + JSON of every handheld shot's active take + a README">Download all (zip)</button>
			</div>
		</div>
	</fieldset>
	{#if out.busy}<p class="hint" role="status">{out.busy}</p>{/if}
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-15);
		padding: var(--space-075) var(--space-1) var(--space-15);
		align-content: start;
	}
	/* fieldset + legend are tools.css's (no frame, the legend a caption); the panel's gap spaces the groups */
	fieldset {
		margin: 0;
	}
	.rows {
		display: grid;
		gap: var(--space-05);
		min-width: 0;
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
