<script lang="ts">
	import { FX_IMPACTS, FX_MAX_DURATION, FX_SOUNDS, type Path } from '@scutoons/content';
	import { impactCue } from '@scutoons/reader';
	import { Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * fx: an impact (agents/FX.md) – shake, flash, zoom punch or vignette on the slice in view or the whole screen, with
	 * its own sound (or another / none). ▶ plays it on the canvas.
	 */
	let { doc, path, event }: { doc: ContentDoc; path: Path; event: any } = $props();

	const effects = () => FX_IMPACTS.map((e) => ({ value: e, label: m[`creator.fx_effect_${e}`]() }));
	const targets = () => [{ value: 'screen', label: m['creator.fx_target_screen']() }];
	const sounds = () => [{ value: 'none', label: m['creator.fx_sound_none']() }, ...FX_SOUNDS.map((s) => ({ value: s, label: m[`creator.fx_sound_${s}`]() }))];

	function preview() {
		const p = event?.params ?? {};
		if (!(FX_IMPACTS as readonly string[]).includes(p.effect)) return;
		const canvas = document.querySelector<HTMLElement>('[data-testid="slice-canvas"]');
		impactCue(p.effect, {
			slice: p.target === 'screen' ? null : canvas,
			screen: canvas,
			sound: p.sound === 'none' ? null : p.sound,
			duration: Number(p.duration) > 0 ? Math.min(FX_MAX_DURATION, Number(p.duration)) : undefined
		});
	}
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'params', 'effect']} label={m['creator.fx_effect']()} type="select" options={effects()} />
	<PropField {doc} path={[...path, 'params', 'target']} label={m['creator.fx_target']()} type="select" options={targets()} placeholder={m['creator.fx_target_slice']()} />
	<PropField {doc} path={[...path, 'params', 'sound']} label={m['creator.fx_sound']()} type="select" options={sounds()} placeholder={m['creator.fx_sound_own']()} />
	<PropField {doc} path={[...path, 'params', 'duration']} label={m['creator.field_duration']()} type="number" step={50} placeholder={m['creator.fx_duration_own']()} />
	<PropField {doc} path={[...path, 'params', 'wait']} label={m['creator.vn_wait']()} type="bool" placeholder="false" />
	<button type="button" class="sc-round" title={m['creator.fx_preview']()} aria-label={m['creator.fx_preview']()} onclick={preview} data-testid="fx-preview"><Icon name="play" /></button>
</div>
