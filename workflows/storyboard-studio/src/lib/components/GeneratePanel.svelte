<script>
	// Generate a shot with a pluggable generator (owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit
	// bauen, dass ich die Runway API anbinden kann."): pick the generator + model, its inputs pre-filled from the shot,
	// the estimate before anything is sent (a paid one asks with the cost), the shot's jobs with progress + cancel; the
	// results land in Variants → Generated. README "Generators", generators/jobs.mjs.
	import { untrack } from 'svelte';
	import { Icon, IconButton } from '@scutoons/ui';
	import { fail, studio } from '../studio.svelte.js';
	import { cancel, costText, current, estimate, gen, loadGenerators, loadInputs, model, setGenerator, setModel, start, wants } from '../generate.svelte.js';

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
	const blocked = $derived(!g?.configured || !m || !!gen.estimate?.error || gen.busy || (wants('still') && m?.mode === 'image_to_video' && !gen.form.still) || (wants('control') && !gen.form.control));
</script>

{#if id}
	<div class="panel">
		<fieldset>
			<legend>Generator</legend>
			<div class="sc-fields" data-cols="1" data-density="panel">
				<label class="field">Generator
					<select value={gen.form.generator} onchange={(e) => setGenerator(e.currentTarget.value)}>
						{#each gen.generators as x (x.id)}<option value={x.id} selected={x.id === gen.form.generator}>{x.label}{x.configured ? '' : ' – not configured'}</option>{/each}
					</select>
				</label>
				{#if g?.models.length}
					<label class="field">Model
						<select value={gen.form.model} onchange={(e) => setModel(e.currentTarget.value)}>
							{#each g.models as x (x.id)}<option value={x.id} selected={x.id === gen.form.model}>{x.label}</option>{/each}
						</select>
					</label>
				{/if}
			</div>
			{#if g && !g.configured}<p class="hint">{g.how}</p>{:else if g?.note}<p class="hint muted">{g.note}</p>{/if}
		</fieldset>

		{#if g?.configured && m}
			<fieldset>
				<legend>Inputs</legend>
				{#if !m.inputs.length}<p class="hint muted">Takes the shot's render and character images itself.</p>{/if}
				{#if wants('still')}
					<label class="field">{m.mode === 'video_to_video' ? 'Keyframe at the start (optional)' : 'Still (the first frame)'}
						<select bind:value={gen.form.still}>
							{#if m.mode === 'video_to_video'}<option value="">None</option>{/if}
							{#each gen.inputs?.stills ?? [] as s (s.key)}<option value={s.key} selected={s.key === gen.form.still}>{s.label}</option>{/each}
						</select>
					</label>
					{#if still?.url}<img class="thumb" src={`${still.url}?v=${v}`} alt="" />{/if}
				{/if}
				{#if wants('control')}
					<label class="field">Handheld render pass
						<select bind:value={gen.form.control}>
							{#each gen.inputs?.controls ?? [] as c (c.key)}<option value={c.key} selected={c.key === gen.form.control}>{c.label}</option>{/each}
						</select>
					</label>
					{#if !gen.inputs?.controls.length}<p class="hint">No render pass yet – Handheld → "Render all handheld shots".</p>{/if}
				{/if}
				{#if wants('prompt')}
					<label class="field">Prompt <textarea rows="4" maxlength="1000" bind:value={gen.form.prompt}></textarea></label>
				{/if}
				{#if wants('duration')}
					<label class="field">Seconds <input type="number" min={m.durations?.min ?? 1} max={m.durations?.max ?? 10} step="1" bind:value={gen.form.duration} /></label>
				{/if}
			</fieldset>
			<p class="cost" role="status" class:sc-error={gen.estimate?.error}>{costText(gen.estimate)}</p>
			<div class="buttons">
				<button type="button" onclick={start} disabled={blocked} title={g.paid ? 'Asks with the cost before anything is sent' : ''}>Generate</button>
			</div>
		{/if}

		{#if gen.jobs.length || gen.spend.jobs}
			<fieldset>
				<legend>Jobs</legend>
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
	.field {
		display: grid;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
		min-width: 0;
	}
	.field :is(select, input, textarea) {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
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
	progress {
		width: 100%;
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
