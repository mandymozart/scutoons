<script lang="ts">
	// The handheld camera of the current shot (#126; owner 2026-10-19: "In the storyboard module I'd enable it per shot … on
	// the PC the camera of that shot … moves along with the phone and records the camera movement. Afterwards I only need
	// to trim"; the studio's HandheldPanel): the switch per shot, the phone (phone-pairing), the takes (inline in the shot,
	// one active – the radio –, the bin), the active take's smoothing and scale, its trim (the timeline in Take mode) and
	// Render (the Sin City look as a WebM into the workspace folder + its record). Every change is one undo step.
	import { confirmDialog, Icon, IconButton, PropRow, ScrubNumber, Segmented } from '@scutoons/ui';
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
			toast(m['storyboard.hh_rendered']({ take: take.id, shot: entry.id }), { type: 'success' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
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
		<PropRow label={m['storyboard.hh_switch']()} hint={m['storyboard.hh_switch_hint']()}>
			<Segmented
				label={m['storyboard.hh_switch_label']({ id: entry.id })}
				stretch
				options={[
					{ value: 'off', label: m['storyboard.hh_off']() },
					{ value: 'on', label: m['storyboard.hh_on']({ id: entry.id }) }
				]}
				value={doc.on ? 'on' : 'off'}
				onchange={(v) => {
					if (v === 'off') sb.hh.stopLive();
					change(setOn(v === 'on'));
				}}
				testid="hh-switch"
			/>
		</PropRow>
		{#if doc.on}
			<fieldset>
				<legend>{m['storyboard.hh_phone']()}</legend>
				<div class="rows"><PhonePairing {sb} /></div>
			</fieldset>
			{#if doc.takes.length}
				<fieldset>
					<legend>{m['storyboard.hh_takes']()}</legend>
					<div class="rows">
						<div class="takes" role="radiogroup" aria-label={m['storyboard.hh_active_take']()}>
							{#each doc.takes as t (t.id)}
								<div class="take">
									<button type="button" class="secondary" class:sc-selected={doc.active === t.id} role="radio" aria-checked={doc.active === t.id} onclick={() => change(setActive(t.id))} data-testid="hh-take-{t.id}">
										<span class="cut">{m['storyboard.hh_take_row']({ take: t.id, kept: (t.out - t.in).toFixed(1), all: takeEnd(t).toFixed(1), at: when(t.created) })}</span>
									</button>
									<IconButton label={m['storyboard.hh_delete_take']({ take: t.id })} class="sc-round" onclick={() => remove(t.id, t.out - t.in, t.samples.length)}><Icon name="trash-2" /></IconButton>
								</div>
							{/each}
						</div>
						{#if take}
							<PropRow label={m['storyboard.hh_trim']()} hint={m['storyboard.hh_trim_hint']()}>
								{#if trimHere}
									<p class="sc-caption">{m['storyboard.hh_trim_here']({ take: take.id })}</p>
								{:else}
									<button type="button" class="secondary" onclick={toTrim} data-testid="hh-to-trim">{m['storyboard.hh_open_timeline']()}</button>
								{/if}
							</PropRow>
							<PropRow label={m['storyboard.hh_smooth']()} hint={m['storyboard.hh_smooth_hint']()}>
								<ScrubNumber label={m['storyboard.hh_smooth_take']({ take: take.id })} icon="wind" min={0} max={1} step={0.05} required value={take.smooth ?? 0} onchange={(v) => change(setTakeField(take.id, 'smooth', v ?? 0), `${entry.id}:${take.id}:smooth`)} />
							</PropRow>
							<PropRow label={m['storyboard.hh_render']()} hint={m['storyboard.hh_render_hint']()}>
								<button type="button" onclick={render} disabled={!!sb.hh.rendering || sb.workspaceState !== 'ready'} title={sb.workspaceState === 'ready' ? m['storyboard.hh_render_hint']() : m['storyboard.hh_render_folder']()} data-testid="hh-render">
									{sb.hh.rendering ?? m['storyboard.hh_render_take']({ take: take.id })}
								</button>
							</PropRow>
						{/if}
					</div>
				</fieldset>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
		padding: var(--space-1);
	}
	.rows {
		display: grid;
		gap: var(--space-05);
	}
	.takes {
		display: grid;
		gap: var(--space-025);
	}
	.take {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.take button.secondary {
		flex: 1;
		min-width: 0;
		justify-content: flex-start;
	}
	.cut {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	p {
		margin: 0;
	}
</style>
