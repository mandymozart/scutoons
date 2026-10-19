<script>
	// The handheld camera of a shot (owner 2026-10-19: "In the storyboard module I'd enable it per shot … on the PC the
	// camera of that shot … moves along with the phone and records the camera movement. Afterwards I only need to trim"):
	// the switch per shot, the phone (PhonePairing), the takes (one active, its smoothing), the take's downloads. The trim
	// is the timeline's Take lane (Timeline.svelte); rendering all handheld shots + the zip are the Export step's (Output
	// panel). README "Handheld camera", research/HANDHELD-CAMERA.md. The takes file follows the selection in every step
	// (handheld.svelte.js followSelection).
	import { confirmDialog, Icon, IconButton, PropRow, ScrubNumber, Segmented } from '@scutoons/ui';
	import { fail, find, studio } from '../studio.svelte.js';
	import { activeTake, endPreview, hh, preview, saveDoc, setOn } from '../handheld/handheld.svelte.js';
	import { downloadTake } from '../handheld/export.js';
	import PhonePairing from './PhonePairing.svelte';

	const id = $derived(studio.sel?.id);
	const doc = $derived(hh.doc && hh.docShot === id ? hh.doc : null);
	const take = $derived(doc ? activeTake() : null);
	let busy = $state('');

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
	async function one(kind, withScene = false) {
		busy = 'Exporting …';
		try { await downloadTake(find(id), id, $state.snapshot(take), kind, withScene); } catch (e) { fail(e); } finally { busy = ''; }
	}
</script>

<div class="panel">
	{#if id && doc}
		<PropRow label="Handheld" hint="The phone as this shot's camera">
			<Segmented label="Handheld camera for {id}" stretch options={[{ value: 'off', label: 'Off' }, { value: 'on', label: `On for ${id}` }]} value={doc.on ? 'on' : 'off'} onchange={(v) => setOn(v === 'on')} />
		</PropRow>
		{#if doc.on}
			<fieldset>
				<legend>Phone</legend>
				<PhonePairing />
			</fieldset>
			{#if doc.takes.length}
				<fieldset>
					<legend>Takes</legend>
					<div class="takes" role="radiogroup" aria-label="The active take">
						{#each doc.takes as t (t.id)}
							<div class="take">
								<button type="button" class="secondary" class:sc-selected={doc.active === t.id} role="radio" aria-checked={doc.active === t.id} onclick={() => pick(t.id)}>{t.id} · {(t.out - t.in).toFixed(1)} s of {t.samples.at(-1)?.t.toFixed(1)} s · {t.created.slice(11, 16)}</button>
								<IconButton label={`Delete take ${t.id}`} class="sc-round" onclick={() => remove(t)}><Icon name="trash-2" /></IconButton>
							</div>
						{/each}
					</div>
					{#if take}
						<p class="hint">Trim {take.id} on the timeline below – drag its in / out handles; the 3D view shows that moment.</p>
						<PropRow label="Smooth" hint="0 = the raw handheld look … 1 = a slow steadicam">
							<ScrubNumber label="Smoothing of {take.id}" icon="wind" min={0} max={1} step={0.05} required value={take.smooth ?? 0} onchange={(v) => { take.smooth = v ?? 0; preview(hh.playhead); saveDoc(true); }} />
						</PropRow>
						<PropRow label="Download" hint="The take's camera for other tools (README: Blender, Unreal)">
							<button type="button" class="secondary" onclick={() => one('json')} disabled={!!busy} title="The samples (t, position, quaternion, fov)">JSON</button>
							<button type="button" class="secondary" onclick={() => one('glb')} disabled={!!busy} title="An animated camera for Blender (File → Import → glTF 2.0) / Unreal (glTF importer → Sequencer)">glTF</button>
							<button type="button" class="secondary" onclick={() => one('glb', true)} disabled={!!busy} title="The animated camera + the blocked set and figures">glTF + scene</button>
						</PropRow>
						{#if busy}<p class="hint" role="status">{busy}</p>{/if}
					{/if}
				</fieldset>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
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
	.takes {
		display: grid;
		gap: var(--space-025);
	}
	.take {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.take button[role='radio'] {
		flex: 1;
		justify-content: flex-start;
		font-size: var(--type-caption-size);
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
</style>
