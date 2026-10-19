<script>
	// Generate a shot with a pluggable generator (owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit
	// bauen, dass ich die Runway API anbinden kann."): pick the generator + model, its inputs pre-filled from the shot,
	// the estimate before anything is sent (a paid one asks with the cost), the shot's jobs with progress + cancel; the
	// results land in Variants → Generated. "Build from the shot" fills the prompt with everything set for the shot; Luma –
	// by hand (transport 'manual') shows it to copy instead of Generate (ManualGenerate – the former Luma panel, owner
	// 2026-10-19). README "Generators", generators/jobs.mjs.
	import { untrack } from 'svelte';
	import { Icon, IconButton, PropRow, PropSelect, ScrubNumber } from '@scutoons/ui';
	import { fail, studio } from '../studio.svelte.js';
	import { buildPrompt, cancel, costText, current, estimate, gen, loadGenerators, loadInputs, model, setGenerator, setModel, start, wants } from '../generate.svelte.js';
	import ManualGenerate from './ManualGenerate.svelte';

	const id = $derived(studio.sel?.id);
	const v = $derived(studio.version);
	$effect(() => { if (!gen.generators.length) loadGenerators().catch(fail); });
	// a new shot pre-fills the form; a reload (a new variant) only refreshes the lists
	$effect(() => { void v; const shot = id; if (shot) untrack(() => loadInputs(shot).catch(fail)); });
	// the estimate follows every change of the form
	$effect(() => { void JSON.stringify(gen.form); void gen.inputs; estimate(); });
	const g = $derived(current());
	const m = $derived(model());
	const still = $derived(gen.inputs?.stills.find((s) => s.key === gen.form.still));
	const active = (j) => j.status === 'queued' || j.status === 'running';
	// by hand (transport 'manual' – generators/jobs.mjs): nothing is sent, the panel shows the prompt to copy
	const manual = $derived(g?.transport === 'manual');
	const built = $derived(!!id && gen.built?.shot === id);
	/** Runway reads the first 1000 characters of a prompt (runway.mjs) – a built prompt is longer: said, not cut */
	const PROMPT_MAX = 1000;
	const blocked = $derived(!g?.configured || !m || !!gen.estimate?.error || gen.busy || (wants('still') && m?.mode === 'image_to_video' && !gen.form.still) || (wants('control') && !gen.form.control));
</script>

{#if id}
	<div class="panel">
		<!-- the panel's head names it (Generate) and the rows their own: no legend that only repeats "Generator". The rows
		     sit in a plain grid, never a grid fieldset – Chromium leaves a container-query row (PropRow) inside one at 0 px
		     when its options arrive after the mount (review H1: the Generator row was invisible on the first open) -->
		<div class="rows">
			{#if gen.generators.length}
				<PropRow label="Generator"><PropSelect label="Generator" required options={gen.generators.map((x) => ({ value: x.id, label: `${x.label}${x.configured ? '' : ' – not configured'}` }))} value={gen.form.generator} onchange={(v) => setGenerator(v)} /></PropRow>
			{/if}
			{#if g?.models.length}
				<PropRow label="Model"><PropSelect label="Model" required options={g.models.map((x) => ({ value: x.id, label: x.label }))} value={gen.form.model} onchange={(v) => setModel(v)} /></PropRow>
			{/if}
			{#if g && !g.configured}<p class="hint">{g.how}</p>{:else if g?.note}<p class="hint muted">{g.note}</p>{/if}
		</div>

		{#if g?.configured && m}
			<fieldset>
				<legend>Inputs</legend>
				<div class="rows">
					{#if !m.inputs.length}<p class="hint muted">Takes the shot's render and character images itself.</p>{/if}
					{#if wants('still')}
						<!-- a still the model needs keeps an empty "(pick one)" while none is picked (after "(none)" as a keyframe):
						     without it the select would show its first still as if chosen, and Generate stays off unexplained -->
						<PropRow label={m.mode === 'video_to_video' ? 'Keyframe' : 'Still'} hint={m.mode === 'video_to_video' ? 'A keyframe at the start (optional)' : 'The first frame'}>
							<PropSelect label={m.mode === 'video_to_video' ? 'Keyframe at the start (optional)' : 'Still (the first frame)'} required={m.mode !== 'video_to_video' && !!gen.form.still} placeholder={m.mode === 'video_to_video' ? 'none' : 'pick one'} options={(gen.inputs?.stills ?? []).map((x) => ({ value: x.key, label: x.label }))} value={gen.form.still || undefined} onchange={(v) => (gen.form.still = v ?? '')} />
						</PropRow>
						{#if still?.url}<img class="thumb" src={`${still.url}?v=${v}`} alt="" />{/if}
					{/if}
					{#if wants('control')}
						<PropRow label="Pass" hint="The handheld render pass the video follows">
							<PropSelect label="Handheld render pass" required options={(gen.inputs?.controls ?? []).map((x) => ({ value: x.key, label: x.label }))} value={gen.form.control || undefined} onchange={(v) => (gen.form.control = v ?? '')} />
						</PropRow>
						{#if !gen.inputs?.controls.length}<p class="hint">No render pass yet – Export → Output → "Render all handheld shots".</p>{/if}
					{/if}
					{#if wants('prompt')}
						<!-- by hand the built prompt is only shown to copy (its changes belong in the shot's fields) and kept apart
						     from the other generators' prompt; until it is built, Build is the panel's one primary (then Copy
						     prompt – ManualGenerate) -->
						<PropRow label="Prompt">
							<button type="button" class:secondary={!manual || built} onclick={() => buildPrompt({ fill: !manual }).catch(fail)} disabled={gen.building} title="Everything set for this shot – also unsaved changes – as one prompt{manual ? '' : ' (replaces the text below)'}">Build from the shot</button>
							{#if !manual}
								<textarea data-prop-line aria-label="Prompt" rows="4" bind:value={gen.form.prompt}></textarea>
							{:else if built}
								<textarea data-prop-line aria-label="Prompt" readonly rows="16" value={gen.built?.text ?? ''}></textarea>
							{/if}
						</PropRow>
						{#if !manual && gen.form.prompt?.length > PROMPT_MAX}<p class="hint">{gen.form.prompt.length} characters – only the first {PROMPT_MAX} are sent.</p>{/if}
					{/if}
					{#if wants('duration')}
						<PropRow label="Length"><ScrubNumber label="Seconds" icon="clock" unit="s" required min={m.durations?.min ?? 1} max={m.durations?.max ?? 10} step={1} value={gen.form.duration} onchange={(v) => (gen.form.duration = Math.round(v ?? m.durations?.min ?? 1))} /></PropRow>
					{/if}
				</div>
			</fieldset>
			{#if manual}
				<ManualGenerate {built} />
			{:else}
				<p class="cost" role="status" class:sc-error={gen.estimate?.error}>{costText(gen.estimate)}</p>
				<div class="buttons">
					<button type="button" onclick={start} disabled={blocked} title={g.paid ? 'Asks with the cost before anything is sent' : ''}>Generate</button>
				</div>
			{/if}
		{/if}

		{#if gen.jobs.length || gen.spend.jobs}
			<fieldset>
				<legend>Jobs</legend>
				<div class="rows">
					<ul class="jobs">
						{#each gen.jobs as j (j.id)}
							<li>
								<div class="row">
									<span>{j.model ?? j.generator} · {j.status}{j.cost ? ` · ${j.cost.usd.toFixed(2)} USD` : j.estimate?.usd ? ` · ≈ ${j.estimate.usd.toFixed(2)} USD` : ''}</span>
									{#if active(j)}<IconButton label={`Cancel ${j.id}`} class="sc-round" onclick={() => cancel(j.id)}><Icon name="x" /></IconButton>{/if}
								</div>
								{#if active(j)}<progress max="1" value={j.progress ?? undefined}></progress>{/if}
								{#if j.message}<p class="hint" class:sc-error={j.status === 'failed'}>{j.message}</p>{/if}
							</li>
						{/each}
					</ul>
					<p class="hint muted">This session: {gen.spend.usd.toFixed(2)} USD · {gen.spend.jobs} paid job{gen.spend.jobs === 1 ? '' : 's'}</p>
				</div>
			</fieldset>
		{/if}
	</div>
{/if}

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
		padding: var(--space-1);
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
	textarea {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		resize: vertical;
	}
	/* the built prompt to copy: long – caption size */
	textarea[readonly] {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.thumb {
		width: 100%;
		max-height: 12rem;
		object-fit: contain;
		background: var(--ink);
		display: block;
	}
	.jobs {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-05);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
	}
	/* the house bar (@scutoons/ui BulkProgress: ink on paper in an ink frame), never the browser's rounded blue one –
	   owner 2026-10-19: "Use the dogma from the creator"; no progress reported yet (queued) = the empty frame */
	progress {
		display: block;
		box-sizing: border-box;
		width: 100%;
		height: 1rem;
		appearance: none;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		background: var(--paper);
		color: var(--ink);
	}
	progress::-webkit-progress-bar {
		background: var(--paper);
	}
	progress::-webkit-progress-value {
		background: var(--ink);
	}
	progress::-moz-progress-bar {
		background: var(--ink);
	}
	progress:indeterminate::-moz-progress-bar {
		background: none;
	}
	.buttons {
		display: flex;
		justify-content: flex-end;
	}
	.cost,
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
