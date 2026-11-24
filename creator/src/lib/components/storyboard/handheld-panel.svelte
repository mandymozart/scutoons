<script lang="ts">
	// The handheld camera of the current shot (#126; lap 2 C3b): the switch per shot, the phone (phone-pairing), the takes
	// (inline in the shot, one active – the radio –, the bin), the active take's trim (the timeline in Take mode; the
	// explaining sentence is hover text), its smoothing and Render (the studio look as a WebM into the workspace folder +
	// its record). Every change is one undo step.
	import { confirmDialog, Segmented } from '@scutoons/ui';
	import { Button, IconButton, Slider } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { activeTake, removeTake, setActive, setOn, setTakeField, takeEnd, takesOf } from '$lib/editor/storyboard-takes.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import PhonePairing from '$lib/components/storyboard/phone-pairing.svelte';

	let { sb }: { sb: StoryboardPage } = $props();

	const entry = $derived(sb.sel);
	const doc = $derived(takesOf(entry?.shot));
	const take = $derived(activeTake(doc));
	// the trim is the timeline's, in Take mode – here: the Handheld step with the timeline open
	const trimHere = $derived(sb.step === 'handheld' && !sb.tl.here.folded && sb.tl.here.mode === 'take');
	const change = (fn: Parameters<typeof sb.editShot>[0], key?: string) => entry && sb.hh.editShot(entry, fn, key);

	function toTrim() {
		sb.setStep('handheld');
		sb.tl.setMode('take');
		sb.tl.setFolded(false);
	}
	async function remove(id: string, kept: number, samples: number) {
		const ok = await confirmDialog({ title: m['storyboard.hh_delete_title']({ take: id }), message: m['storyboard.hh_delete_message']({ s: kept.toFixed(1), n: samples }), confirmLabel: m['storyboard.hh_delete']() });
		if (!ok) return;
		sb.hh.endPreview();
		change(removeTake(id));
	}
	async function render() {
		if (!entry || !take) return;
		try {
			await sb.hh.render(entry, take);
			toast(m['storyboard.hh_rendered']({ take: take.id, shot: entry.id }), { type: 'success', kind: 'generation' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error', kind: 'generation' });
		}
	}
	/** the take's time of day here (hh:mm) */
	const when = (iso: string) => {
		const d = new Date(iso);
		return Number.isNaN(d.getTime()) ? '' : `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
	};
</script>

<div class="panel" data-testid="handheld-panel">
	{#if entry}
		<Segmented
			label={m['storyboard.hh_switch_label']({ id: entry.id })}
			stretch
			options={[
				{ value: 'off', label: m['storyboard.hh_off'](), title: m['storyboard.hh_switch_hint']() },
				{ value: 'on', label: m['storyboard.hh_on']({ id: entry.id }), title: m['storyboard.hh_switch_hint']() }
			]}
			value={doc.on ? 'on' : 'off'}
			onchange={(v) => {
				if (v === 'off') sb.hh.stopLive();
				change(setOn(v === 'on'));
			}}
			testid="hh-switch"
		/>
		{#if doc.on}
			<span class="sc-label">{m['storyboard.hh_phone']()}</span>
			<div><PhonePairing {sb} /></div>
			{#if doc.takes.length}
				<span class="sc-label">{m['storyboard.hh_takes']()}</span>
				<div class="takes" role="radiogroup" aria-label={m['storyboard.hh_active_take']()}>
					{#each doc.takes as t (t.id)}
						<div class="take" class:active={doc.active === t.id}>
							<button type="button" class="pick" role="radio" aria-checked={doc.active === t.id} data-control-exempt="a take's row" onclick={() => change(setActive(t.id))} data-testid="hh-take-{t.id}">
								<span class="id">{t.id}</span>
								<span class="kept">{m['storyboard.hh_take_kept']({ kept: (t.out - t.in).toFixed(1), all: takeEnd(t).toFixed(1) })}</span>
								<span class="at">{when(t.created)}</span>
							</button>
							<IconButton label={m['storyboard.hh_delete_take']({ take: t.id })} icon="trash-2" tone="muted" onclick={() => remove(t.id, t.out - t.in, t.samples.length)} />
						</div>
					{/each}
				</div>
				{#if take}
					{#if !trimHere}
						<span class="sc-label" title={m['storyboard.hh_trim_hint']()}>{m['storyboard.hh_trim']()}</span>
						<div><Button onclick={toTrim} title={m['storyboard.hh_trim_here']({ take: take.id })} data-testid="hh-to-trim">{m['storyboard.hh_open_timeline']()}</Button></div>
					{/if}
					<label class="sc-label" for="hh-take-smooth" title={m['storyboard.hh_smooth_hint']()}>{m['storyboard.hh_smooth']()}</label>
					<Slider
						id="hh-take-smooth"
						min={0}
						max={1}
						step={0.05}
						value={take.smooth ?? 0}
						label={m['storyboard.hh_smooth_take']({ take: take.id })}
						title="{m['storyboard.hh_smooth_take']({ take: take.id })}: {(take.smooth ?? 0).toFixed(2)}"
						oninput={(v) => change(setTakeField(take.id, 'smooth', v), `${entry.id}:${take.id}:smooth`)}
						data-testid="hh-take-smooth"
					/>
					<span class="sc-label" title={m['storyboard.hh_render_hint']()}>{m['storyboard.hh_render']()}</span>
					<div>
						<Button onclick={render} disabled={!!sb.hh.rendering || sb.workspaceState !== 'ready'} title={sb.workspaceState === 'ready' ? m['storyboard.hh_render_hint']() : m['storyboard.hh_render_folder']()} data-testid="hh-render">
							{sb.hh.rendering ?? m['storyboard.hh_render_take']({ take: take.id })}
						</Button>
					</div>
				{/if}
			{/if}
		{/if}
	{/if}
</div>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		padding: var(--editor-pad);
		--slider-length: 100%;
	}
	.sc-label {
		margin-top: 0.7rem;
	}
	.sc-label:first-child {
		margin-top: 0;
	}
	.takes {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	.take {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		border-radius: var(--editor-radius);
	}
	.take.active {
		background: var(--kit-list-row-selected-ground);
		color: var(--kit-list-row-selected-text);
	}
	.pick {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-row);
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	@media (hover: hover) {
		.take:not(.active):hover {
			background: var(--kit-list-row-hover-ground);
		}
	}
	.pick:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	.id {
		font-family: var(--editor-sans);
	}
	.kept {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.kept,
	.at {
		font-size: var(--editor-small);
		color: var(--tool-muted);
	}
	.take.active .kept,
	.take.active .at {
		color: inherit;
	}
</style>
