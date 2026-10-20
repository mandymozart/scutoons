<script lang="ts">
	// The current shot's fields as a compact inspector (DESIGN "Compact inspector"; the studio's ShotPanel,
	// workflows/scenes/storyboard.md "A shot"): one tab at a time – Shot (what it shows), Camera (format + camera), Light,
	// Figures (figures-inspector.svelte); the step remembers the tab (storyboard-steps.ts). Rows = PropRow, numbers =
	// ScrubNumber, small choices = Segmented, the rest PropSelect. Every change is an edit of the chapter's storyboard
	// (undo, autosave, Save draft in the ActionBar); the words are @scutoons/content/storyboard's vocabulary.
	import { PropRow, PropSelect, ScrubNumber, Segmented } from '@scutoons/ui';
	import { ANGLES, FORMATS, LIGHT, MOVES, PROJECTIONS, SIZES, SOURCES, VIEWS } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { ACCENTS, aspectOptions, optionsOf } from '$lib/editor/storyboard-shots.ts';
	import type { ShotTab } from '$lib/editor/storyboard-steps.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import FiguresInspector from '$lib/components/storyboard/figures-inspector.svelte';

	let { sb, tab }: { sb: StoryboardPage; tab: ShotTab } = $props();

	const s = $derived(sb.sel?.shot);
	const sceneLight = $derived(sb.sel?.scene.light ?? {});
	const set = (keys: string[], value: unknown) => sb.setShot(keys, value);
	const aspectWords = { slices: (count: number) => m['storyboard.aspect_slices']({ count }), get offGrid() { return m['storyboard.aspect_off_grid'](); } };
	const text = (e: Event) => (e.currentTarget as HTMLInputElement | HTMLTextAreaElement).value;
</script>

{#if s}
	<div class="fields" data-testid="shot-inspector">
		{#if tab === 'shot'}
			<PropRow label={m['storyboard.action']()} hint={m['storyboard.action_hint']()}>
				<textarea aria-label={m['storyboard.action_hint']()} rows="4" value={s.action?.de ?? ''} oninput={(e) => set(['action', 'de'], text(e))} data-testid="shot-action"></textarea>
			</PropRow>
			<PropRow label={m['storyboard.image']()} hint={m['storyboard.image_hint']()}>
				<textarea aria-label={m['storyboard.image_hint']()} rows="3" value={s.prompt?.en ?? ''} oninput={(e) => set(['prompt', 'en'], text(e))}></textarea>
			</PropRow>
			<PropRow label={m['storyboard.view']()} hint={m['storyboard.view_hint']()}>
				<Segmented label={m['storyboard.view']()} stretch options={VIEWS.map((v) => ({ value: v, label: v }))} value={s.view ?? null} onchange={(v) => set(['view'], v)} onclear={() => set(['view'], null)} />
			</PropRow>
			<PropRow label={m['storyboard.source']()} hint={m['storyboard.source_hint']()}>
				<PropSelect label={m['storyboard.source']()} options={optionsOf(SOURCES, s.source?.want)} value={s.source?.want} onchange={(v) => set(['source', 'want'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.note']()}>
				<textarea aria-label={m['storyboard.note']()} rows="2" value={s.note ?? ''} oninput={(e) => set(['note'], text(e))}></textarea>
			</PropRow>
		{:else if tab === 'camera'}
			<PropRow label={m['storyboard.format']()}>
				<PropSelect label={m['storyboard.format']()} required options={optionsOf(FORMATS, s.format)} value={s.format} onchange={(v) => set(['format'], v)} />
				<PropSelect label={m['storyboard.aspect']()} required options={aspectOptions(s.format, s.aspect, aspectWords)} value={s.aspect} onchange={(v) => set(['aspect'], v)} testid="shot-aspect" />
			</PropRow>
			<PropRow label={m['storyboard.scroll']()} hint={m['storyboard.scroll_hint']()}>
				<PropSelect label={m['storyboard.projection']()} options={optionsOf(PROJECTIONS, s.projection)} value={s.projection} onchange={(v) => set(['projection'], v)} />
				<PropSelect label={m['storyboard.move']()} options={optionsOf(MOVES, s.move)} value={s.move} onchange={(v) => set(['move'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.camera_shot']()} hint={m['storyboard.camera_shot_hint']()}>
				<PropSelect label={m['storyboard.size']()} options={optionsOf(SIZES, s.shot?.size)} value={s.shot?.size} onchange={(v) => set(['shot', 'size'], v)} />
				<PropSelect label={m['storyboard.angle']()} options={optionsOf(ANGLES, s.shot?.angle)} value={s.shot?.angle} onchange={(v) => set(['shot', 'angle'], v)} />
			</PropRow>
			{#if s.camera}
				<PropRow label={m['storyboard.lens']()} hint={m['storyboard.lens_hint']()}>
					<ScrubNumber label={m['storyboard.fov']()} icon="aperture" unit="°" min={5} max={150} step={0.5} value={s.camera.fov} onchange={(v) => set(['camera', 'fov'], v)} />
				</PropRow>
				{#if s.camera.pos}<p class="hint">{m['storyboard.hand_camera']({ pos: s.camera.pos.map((n) => n.toFixed(2)).join(', ') })}</p>{/if}
			{:else}
				<p class="hint">{m['storyboard.preset_camera']()}</p>
			{/if}
		{:else if tab === 'light'}
			<p class="hint">{m['storyboard.scene_light']({ light: [sceneLight.key, sceneLight.height, sceneLight.source].map((v) => v ?? '–').join(' · ') })}</p>
			<PropRow label={m['storyboard.light_from']()}>
				<PropSelect label={m['storyboard.light_key']()} options={optionsOf(LIGHT.key, s.light?.key)} placeholder={m['storyboard.scene_value']()} value={s.light?.key} onchange={(v) => set(['light', 'key'], v)} />
				<PropSelect label={m['storyboard.light_height']()} options={optionsOf(LIGHT.height, s.light?.height)} placeholder={m['storyboard.scene_value']()} value={s.light?.height} onchange={(v) => set(['light', 'height'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.light_source']()}>
				<PropSelect label={m['storyboard.light_source']()} options={optionsOf(LIGHT.source, s.light?.source)} placeholder={m['storyboard.scene_value']()} value={s.light?.source} onchange={(v) => set(['light', 'source'], v)} />
				<PropSelect label={m['storyboard.light_shadow']()} options={optionsOf(LIGHT.shadow, s.light?.shadow)} placeholder={m['storyboard.scene_value']()} value={s.light?.shadow} onchange={(v) => set(['light', 'shadow'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.light_direction']()} hint={m['storyboard.light_direction_hint']()}>
				<ScrubNumber label={m['storyboard.azimuth']()} tag="AZ" unit="°" min={-180} max={180} value={s.light?.azimuth} onchange={(v) => set(['light', 'azimuth'], v)} />
				<ScrubNumber label={m['storyboard.elevation']()} tag="EL" unit="°" min={-40} max={90} value={s.light?.elevation} onchange={(v) => set(['light', 'elevation'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.softness']()} hint={m['storyboard.softness_hint']()}>
				<ScrubNumber label={m['storyboard.softness']()} icon="cloud" min={0} max={1} step={0.05} placeholder="0.5" value={s.light?.soft} onchange={(v) => set(['light', 'soft'], v)} />
			</PropRow>
			<PropRow label={m['storyboard.accent']()}>
				<PropSelect label={m['storyboard.accent_color']()} options={optionsOf(ACCENTS, s.light?.accent?.color)} placeholder={m['storyboard.none_value']()} value={s.light?.accent?.color} onchange={(v) => set(['light', 'accent', 'color'], v)} />
				<Segmented label={m['storyboard.rim']()} options={[{ value: 'rim', label: m['storyboard.rim'](), title: m['storyboard.rim_hint']() }]} value={s.light?.rim ? 'rim' : null} onchange={() => set(['light', 'rim'], true)} onclear={() => set(['light', 'rim'], null)} />
			</PropRow>
			<PropRow label={m['storyboard.note']()} hint={m['storyboard.light_note']()}>
				<input type="text" aria-label={m['storyboard.light_note']()} value={s.light?.note?.de ?? ''} oninput={(e) => set(['light', 'note', 'de'], text(e))} />
			</PropRow>
		{:else}
			<FiguresInspector {sb} />
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
	textarea,
	input[type='text'] {
		width: 100%;
		box-sizing: border-box;
	}
	textarea {
		resize: vertical;
	}
	.hint {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
</style>
