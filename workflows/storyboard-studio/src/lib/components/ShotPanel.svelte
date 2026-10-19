<script>
	// The shot's fields (workflows/scenes/storyboard.md) as a compact inspector (DESIGN "Compact inspector"): one tab at a
	// time – Shot (what it shows), Camera (format + camera), Light, Figures; the step remembers the tab (steps.svelte.js).
	// Rows = PropRow, numbers = ScrubNumber, small choices = Segmented, the rest PropSelect; each change re-renders the 3D
	// view. Save / Keep as variant / Revert / the review are the ActionBar's (StudioActions).
	import { Icon, IconButton, PropRow, PropSelect, ScrubNumber, Segmented } from '@scutoons/ui';
	import { studio, touched, VOCAB } from '../studio.svelte.js';

	/** @type {{ tab: string }} */
	let { tab } = $props();
	const s = $derived(studio.sel?.shot);
	const castRefs = $derived((studio.sel?.scene.cast ?? []).map((c) => c.ref));
	const sceneLight = $derived(studio.sel?.scene.light ?? {});
	const opts = (list) => list.filter(Boolean).map((v) => ({ value: v, label: v }));

	/** a value set, or unset (undefined / '' / null) */
	function set(obj, key, value) {
		if (value === '' || value == null) delete obj[key];
		else obj[key] = value;
		touched();
	}
	function ensure(key, init) {
		s[key] ??= init;
		return s[key];
	}
	const light = (key, value) => set(ensure('light', {}), key, value);
	const shotField = (key, value) => set(ensure('shot', {}), key, value);
</script>

{#if s}
	<div class="fields">
		{#if tab === 'shot'}
			<PropRow label="Action" hint="What happens (de)"><textarea aria-label="Action (de)" rows="4" value={s.action?.de ?? ''} oninput={(e) => { s.action = { ...(s.action ?? {}), de: e.currentTarget.value }; touched(); }}></textarea></PropRow>
			<PropRow label="Image" hint="The picture in English, for the generators' prompts"><textarea aria-label="Image (en, for prompts)" rows="3" value={s.prompt?.en ?? ''} oninput={(e) => set(s, 'prompt', e.currentTarget.value ? { en: e.currentTarget.value } : null)}></textarea></PropRow>
			<PropRow label="View" hint="Whose perspective"><Segmented label="Perspective" stretch options={opts(VOCAB.view)} value={s.view ?? null} onchange={(v) => set(s, 'view', v)} onclear={() => set(s, 'view', null)} /></PropRow>
			<PropRow label="Source" hint="What the final image should be"><PropSelect label="Source" required options={opts(VOCAB.want)} value={s.source?.want} onchange={(v) => { ensure('source', { type: 'schematic' }).want = v; touched(); }} /></PropRow>
			<PropRow label="Note"><textarea aria-label="Note" rows="2" value={s.note ?? ''} oninput={(e) => set(s, 'note', e.currentTarget.value)}></textarea></PropRow>
		{:else if tab === 'camera'}
			<PropRow label="Format"><PropSelect label="Format" required options={opts(VOCAB.format)} value={s.format} onchange={(v) => set(s, 'format', v)} /><PropSelect label="Aspect" required options={opts(VOCAB.aspect)} value={s.aspect} onchange={(v) => set(s, 'aspect', v)} /></PropRow>
			<PropRow label="Scroll" hint="Projection and scroll move"><PropSelect label="Projection" required options={opts(VOCAB.projection)} value={s.projection} onchange={(v) => set(s, 'projection', v)} /><PropSelect label="Scroll move" required options={opts(VOCAB.move)} value={s.move} onchange={(v) => set(s, 'move', v)} /></PropRow>
			<PropRow label="Shot" hint="Size and angle – the shot camera"><PropSelect label="Size" required options={opts(VOCAB.size)} value={s.shot?.size} onchange={(v) => shotField('size', v)} /><PropSelect label="Angle" required options={opts(VOCAB.angle)} value={s.shot?.angle} onchange={(v) => shotField('angle', v)} /></PropRow>
			{#if s.camera}
				<PropRow label="Lens" hint="The hand camera (moved in the 3D view)"><ScrubNumber label="Field of view" icon="aperture" unit="°" min={5} max={150} step={0.5} required value={s.camera.fov} onchange={(v) => { s.camera.fov = v; touched(); }} /></PropRow>
				<p class="hint">Hand camera at {s.camera.pos.map((n) => n.toFixed(2)).join(', ')} – "Shot camera" in the 3D view goes back to size + angle.</p>
			{:else}
				<p class="hint">The camera comes from size + angle – move it in the 3D view to set it by hand.</p>
			{/if}
		{:else if tab === 'light'}
			<p class="hint">Scene: {sceneLight.key ?? '–'} · {sceneLight.height ?? '–'} · {sceneLight.source ?? '–'} – an empty field takes the scene's.</p>
			<PropRow label="From"><PropSelect label="Key from" options={opts(VOCAB.key)} placeholder="scene" value={s.light?.key} onchange={(v) => light('key', v)} /><PropSelect label="Height" options={opts(VOCAB.height)} placeholder="scene" value={s.light?.height} onchange={(v) => light('height', v)} /></PropRow>
			<PropRow label="Source"><PropSelect label="Source" options={opts(VOCAB.source)} placeholder="scene" value={s.light?.source} onchange={(v) => light('source', v)} /><PropSelect label="Shadow" options={opts(VOCAB.shadow)} placeholder="scene" value={s.light?.shadow} onchange={(v) => light('shadow', v)} /></PropRow>
			<PropRow label="Direction" hint="A free direction instead of From: azimuth 0 = from the camera, 90 = right, 180 = behind; elevation"><ScrubNumber label="Azimuth" tag="AZ" unit="°" min={-180} max={180} placeholder="key" value={s.light?.azimuth} onchange={(v) => light('azimuth', v)} /><ScrubNumber label="Elevation" tag="EL" unit="°" min={-40} max={90} placeholder="key" value={s.light?.elevation} onchange={(v) => light('elevation', v)} /></PropRow>
			<PropRow label="Softness" hint="0 = Sin City hard (solid black shadows) … 1 = soft (hatched shadows, a fill)"><ScrubNumber label="Softness" icon="cloud" min={0} max={1} step={0.05} placeholder="0.5" value={s.light?.soft} onchange={(v) => light('soft', v)} /></PropRow>
			<PropRow label="Accent"><PropSelect label="Accent colour" options={opts(VOCAB.accent)} placeholder="none" value={s.light?.accent?.color} onchange={(v) => light('accent', v ? { ...(s.light?.accent ?? {}), color: v } : null)} /><Segmented label="Rim light" options={[{ value: 'rim', label: 'Rim', title: 'A rim light behind the figures' }]} value={s.light?.rim ? 'rim' : null} onchange={() => light('rim', true)} onclear={() => light('rim', null)} /></PropRow>
			<PropRow label="Note" hint="What the light does (de)"><input type="text" aria-label="What the light does (de)" value={s.light?.note?.de ?? ''} oninput={(e) => light('note', e.currentTarget.value ? { de: e.currentTarget.value } : null)} /></PropRow>
		{:else}
			{#each s.figures ?? [] as f, i (i)}
				<div class="figure" class:picked={studio.figure === i}>
					<PropRow label="Figure"><PropSelect label="Cast" required options={opts(castRefs)} value={f.cast} onchange={(v) => set(f, 'cast', v)} /><PropSelect label="Persona" options={opts(VOCAB.persona)} placeholder="cast" value={f.persona} onchange={(v) => set(f, 'persona', v)} /><span class="end"><IconButton label="Remove the figure" class="sc-round" onclick={() => { s.figures.splice(i, 1); touched(); }}><Icon name="trash-2" /></IconButton></span></PropRow>
					<PropRow label="Pose"><PropSelect label="Pose" required options={opts(VOCAB.pose)} value={f.pose} onchange={(v) => set(f, 'pose', v)} /><PropSelect label="Facing" required options={opts(VOCAB.facing)} value={f.facing} onchange={(v) => { set(f, 'facing', v); delete f.turn; }} /></PropRow>
					<PropRow label="Framing"><PropSelect label="Framing" required options={opts(VOCAB.framing)} value={f.framing} onchange={(v) => set(f, 'framing', v)} /><PropSelect label="Mood" options={opts(VOCAB.mood)} placeholder="–" value={f.mood} onchange={(v) => set(f, 'mood', v)} /></PropRow>
					<PropRow label="Place" hint="x / y on the screen (0…1), the height in screens – or drag the figure in the 3D view"><ScrubNumber label="x" tag="X" step={0.01} value={f.x} onchange={(v) => set(f, 'x', v)} /><ScrubNumber label="y" tag="Y" step={0.01} value={f.y} onchange={(v) => set(f, 'y', v)} /><ScrubNumber label="Height (screens)" tag="H" step={0.01} min={0} value={f.h} onchange={(v) => set(f, 'h', v)} /></PropRow>
				</div>
			{/each}
			<div class="add"><button type="button" class="secondary" onclick={() => { s.figures = [...(s.figures ?? []), { cast: castRefs[0], pose: 'stand', facing: 'front', framing: 'full', x: 0.5, y: 0.6, h: 0.3 }]; touched(); }}><Icon name="plus" /> Figure</button></div>
		{/if}
	</div>
{/if}

<style>
	.fields {
		display: grid;
		gap: var(--space-05);
		align-content: start;
		padding: var(--space-075) var(--space-1) var(--space-15);
	}
	textarea {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
	}
	input[type='text'] {
		width: 100%;
		box-sizing: border-box;
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.figure {
		display: grid;
		gap: var(--space-05);
		padding-top: var(--space-05);
		border-top: var(--border-width-regular) solid var(--border-muted);
	}
	.figure:first-child {
		border-top: 0;
		padding-top: 0;
	}
	/* the figure picked in the 3D view */
	.figure.picked {
		border-top-color: var(--ink);
		box-shadow: inset var(--border-width-focus) 0 0 var(--ink);
		padding-left: var(--space-05);
	}
	/* the remove button keeps its round size at the row's end */
	.end {
		flex: none !important;
		display: flex;
	}
	.add {
		display: flex;
		justify-content: flex-end;
	}
</style>
