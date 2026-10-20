<script lang="ts">
	// The Generate panel's form (#121; the studio's GeneratePanel – workflows/storyboard-studio README "Generators"): a
	// generator + model per shot (one without its key shown disabled: "not set up – the admin adds <SETTING>"), "Build
	// from the shot" (the full prompt, editable), the shot's inputs (a still / video of its generations, the figures'
	// character images), the estimate in euro before anything is sent (the provider's figures in the tooltip), Generate →
	// the local references shared, the real body estimated again, an in-page confirmation with the cost, then the paid
	// start – once, never retried. The gate's 402 is one notice. Luma – by hand: generate-manual.svelte.
	import { untrack } from 'svelte';
	import { PropRow, PropSelect, ScrubNumber, confirmDialog } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { generatorState, inputsOf } from '$lib/editor/storyboard-generate.ts';
	import { euro, estimateLine, figuresTitle, money } from '$lib/editor/storyboard-money.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import GenerateManual from '$lib/components/storyboard/generate-manual.svelte';
	import ImproveLine from '$lib/components/storyboard/improve-line.svelte';

	let { sb }: { sb: StoryboardPage } = $props();
	const gen = $derived(sb.gen);

	$effect(() => untrack(() => void gen.load()));
	// the form belongs to the current shot (another one resets it)
	$effect(() => {
		void sb.current;
		untrack(() => gen.follow());
	});

	const g = $derived(gen.current);
	const model = $derived(gen.model);
	const take = $derived(inputsOf(model));
	const rate = $derived(gen.budget?.rate ?? null);
	const generatorOptions = $derived(
		gen.generators.map((x) => {
			const state = generatorState(x);
			const label = x.manual ? m['storyboard.gen_manual']() : state === 'not_configured' ? m['storyboard.gen_not_set_up']({ label: x.label }) : state === 'unavailable' ? m['storyboard.gen_unavailable']({ label: x.label }) : x.label;
			return { value: x.id, label, disabled: state !== 'ready' };
		})
	);
	const local = (id: number) => {
		const x = sb.generations.find((y) => y.id === id);
		return !!x && !x.work_path;
	};
	const genLabel = (x: { id: number; file: string | null; approved: boolean }) => `${x.file?.split('/').pop() ?? `#${x.id}`}${x.approved ? ' ✓' : ''}${local(x.id) ? ` · ${m['storyboard.gen_local']()}` : ''}`;
	const blocked = $derived(!gen.ready || !model || gen.sending || !!gen.estimateError || !gen.estimate || (take.stillRequired && gen.form.still === null) || (model?.refs.required?.includes('control') && gen.form.control === null));
	const budgetLine = $derived(gen.budget && gen.budget.budget > 0 ? m['storyboard.gen_budget_left']({ left: euro(gen.budget.left), budget: euro(gen.budget.budget, { whole: true }) }) : '');
	const noticeText = $derived(
		!gen.notice ? '' : gen.notice.kind === 'none' ? m['storyboard.gen_no_budget']() : m['storyboard.gen_budget_exhausted']({ spent: euro(gen.notice.spent + gen.notice.reserved), budget: euro(gen.notice.budget, { whole: true }) })
	);

	let preparing = $state(false);
	async function send() {
		const at = g;
		if (!at || !model) return;
		preparing = true;
		let ready: Awaited<ReturnType<typeof gen.prepare>>;
		try {
			ready = await gen.prepare();
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
			return;
		} finally {
			preparing = false;
		}
		if (!ready) return;
		const e = ready.estimate;
		const cost = money(e.amount, e.currency, e.budget?.rate ?? rate);
		const b = e.budget ?? gen.budget;
		const ok = await confirmDialog({
			title: m['storyboard.gen_confirm_title']({ generator: at.label }),
			message: m['storyboard.gen_confirm_message']({ model: model.label, cost, figures: figuresTitle(e.amount, e.currency, e.budget?.rate ?? rate, model), left: euro(b?.left ?? 0), budget: euro(b?.budget ?? 0, { whole: true }) }),
			confirmLabel: m['storyboard.gen_confirm_label']({ cost })
		});
		if (!ok) return;
		try {
			const item = await gen.start(ready.body);
			if (item) toast(m['storyboard.gen_started']({ model: model.label }), { type: 'success' });
		} catch (err) {
			toast(err instanceof Error ? err.message : String(err), { type: 'error' });
		}
	}
	const text = (e: Event) => (e.currentTarget as HTMLTextAreaElement).value;
</script>

<div class="form" data-testid="generate-form">
	<ImproveLine {sb} />
	{#if gen.loadError}<p class="hint sc-error">{m['storyboard.gen_load_error']({ error: gen.loadError })}</p>{/if}
	{#if gen.generators.length}
		<PropRow label={m['storyboard.gen_generator']()}>
			<PropSelect label={m['storyboard.gen_generator']()} required options={generatorOptions} value={gen.form.generator} onchange={(v) => v && gen.setGenerator(v)} testid="generate-generator" />
		</PropRow>
		{#if g && !g.manual && g.models.length > 1}
			<PropRow label={m['storyboard.gen_model']()}>
				<PropSelect label={m['storyboard.gen_model']()} required options={g.models.map((x) => ({ value: x.id, label: x.label }))} value={gen.form.model} onchange={(v) => v && gen.setModel(v)} testid="generate-model" />
			</PropRow>
		{/if}
		<!-- every generator the server knows, also the ones without a key: why it can't run, said once -->
		{#each gen.generators.filter((x) => generatorState(x) === 'not_configured') as x (x.id)}
			<p class="hint muted" data-testid="not-set-up-{x.id}">{x.label}: {m['storyboard.gen_not_set_up_hint']({ setting: x.setting })}</p>
		{/each}
	{/if}

	{#if g?.manual}
		<p class="hint muted">{m['storyboard.gen_manual_note']()}</p>
	{/if}

	{#if g && (gen.ready || g.manual)}
		<PropRow label={m['storyboard.gen_prompt']()}>
			<button type="button" class:secondary={!g.manual || !!gen.form.prompt} title={m['storyboard.gen_build_hint']()} onclick={() => gen.build()} data-testid="generate-build">{m['storyboard.gen_build']()}</button>
		</PropRow>
		<!-- the prompt full width under its row (a built one is long); by hand it is only shown to copy -->
		{#if g.manual}
			{#if gen.form.prompt}<textarea aria-label={m['storyboard.gen_prompt']()} readonly rows="12" value={gen.form.prompt} data-testid="generate-prompt"></textarea>{/if}
		{:else}
			<textarea aria-label={m['storyboard.gen_prompt']()} rows="6" value={gen.form.prompt} oninput={(e) => { gen.form.prompt = text(e); gen.estimateSoon(); }} data-testid="generate-prompt"></textarea>
		{/if}
		{#if take.control}
			<PropRow label={m['storyboard.gen_control']()} hint={m['storyboard.gen_control_hint']()}>
				<PropSelect label={m['storyboard.gen_control_hint']()} placeholder={m['storyboard.gen_still_pick']()} options={gen.inputs.controls.map((x) => ({ value: x.id, label: genLabel(x) }))} value={gen.form.control} onchange={(v) => { gen.form.control = v ?? null; gen.estimateSoon(); }} testid="generate-control" />
			</PropRow>
		{/if}
		{#if take.still}
			<PropRow label={m['storyboard.gen_still']()} hint={m['storyboard.gen_still_hint']()}>
				<PropSelect label={m['storyboard.gen_still_hint']()} placeholder={take.stillRequired ? m['storyboard.gen_still_pick']() : m['storyboard.gen_still_none']()} options={gen.inputs.stills.map((x) => ({ value: x.id, label: genLabel(x) }))} value={gen.form.still} onchange={(v) => { gen.form.still = v ?? null; gen.estimateSoon(); }} testid="generate-still" />
			</PropRow>
			{#if gen.form.still !== null && local(gen.form.still)}<p class="hint muted">{m['storyboard.gen_local_ref']()}</p>{/if}
		{/if}
		{#if !g.manual && model?.refs.roles.includes('character') && gen.characters.length}
			<fieldset>
				<legend title={m['storyboard.gen_characters_hint']()}>{m['storyboard.gen_characters']()}</legend>
				{#each gen.characters as c (c.key)}
					<label class="check" title={c.usable ? c.url : m['storyboard.gen_character_http']()}>
						<input type="checkbox" disabled={!c.usable} checked={gen.form.characters.includes(c.key)} onchange={(e) => gen.toggleCharacter(c.key, e.currentTarget.checked)} />
						<a href={c.url} target="_blank" rel="noreferrer">{c.label}</a>
					</label>
				{/each}
			</fieldset>
		{/if}
		{#if take.duration && model?.durations}
			<PropRow label={m['storyboard.gen_length']()}>
				<ScrubNumber label={m['storyboard.gen_seconds']()} icon="clock" unit="s" required min={model.durations.min} max={model.durations.max} step={1} value={gen.form.duration} onchange={(v) => { gen.form.duration = Math.round(v ?? model.durations!.min); gen.estimateSoon(); }} />
			</PropRow>
		{/if}
	{/if}

	{#if g?.manual}
		<GenerateManual {sb} />
	{:else if g && gen.ready}
		<p class="hint" class:sc-error={!!gen.estimateError} role="status" title={gen.estimate ? `${figuresTitle(gen.estimate.amount, gen.estimate.currency, gen.estimate.budget?.rate ?? rate, model)}${gen.estimate.note ? ` · ${gen.estimate.note}` : ''}` : undefined} data-testid="generate-estimate">
			{gen.estimateError ?? (gen.estimate ? estimateLine(gen.estimate, rate) : '')}
		</p>
		{#if budgetLine}<p class="hint muted" data-testid="generate-budget">{budgetLine}</p>{/if}
		{#if noticeText}<p class="hint notice" role="alert" data-testid="generate-notice">{noticeText}</p>{/if}
		<div class="buttons">
			<button type="button" disabled={blocked || preparing} title={m['storyboard.gen_generate_hint']()} onclick={send} data-testid="generate-send">{preparing ? m['storyboard.gen_preparing']() : m['storyboard.gen_generate']()}</button>
		</div>
	{/if}
</div>

<style>
	.form {
		display: grid;
		gap: var(--space-05);
		min-width: 0;
	}
	fieldset {
		margin: 0;
		display: grid;
		gap: var(--space-025);
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
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.muted {
		color: var(--text-muted);
	}
	/* the budget gate's refusal: one framed line that stays until the next send */
	.notice {
		padding: var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
	}
	.check {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.buttons {
		display: flex;
		justify-content: flex-end;
	}
</style>
