<script lang="ts">
	// The current shot's figures (the Shot inspector's Figures tab; the studio's): per figure its cast + persona, pose +
	// facing, framing + mood and its place (x / y on the screen, the height in screens – the 3D view drags it, #124).
	// Add = a figure of the scene's first cast entry; Remove per figure. Every change is an edit of the storyboard.
	import { Icon, IconButton, PropRow, PropSelect, ScrubNumber } from '@scutoons/ui';
	import { FACING, POSES, SIZES, type Figure } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { MOODS, newFigure, optionsOf, PERSONAS } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const figures = $derived(sb.sel?.shot.figures ?? []);
	const castRefs = $derived((sb.sel?.scene.cast ?? []).map((c) => c.ref));
	const set = (i: number, key: keyof Figure, value: unknown) => sb.setShot(['figures', String(i), key], value);
	const add = () => sb.editShot((shot) => (shot.figures = [...(shot.figures ?? []), newFigure(castRefs[0] ?? '') as Figure]));
	const remove = (i: number) =>
		sb.editShot((shot) => {
			shot.figures = (shot.figures ?? []).filter((_, n) => n !== i);
			if (!shot.figures.length) delete shot.figures;
		});
</script>

{#each figures as f, i (i)}
	<div class="figure" data-testid="figure-{i}">
		<PropRow label={m['storyboard.figure']()}>
			<PropSelect label={m['storyboard.cast']()} required options={optionsOf(castRefs, f.cast)} value={f.cast} onchange={(v) => set(i, 'cast', v)} />
			<PropSelect label={m['storyboard.persona']()} options={optionsOf(PERSONAS, f.persona)} placeholder={m['storyboard.cast']()} value={f.persona} onchange={(v) => set(i, 'persona', v)} />
			<span class="end"><IconButton label={m['storyboard.remove_figure']()} class="sc-round" onclick={() => remove(i)}><Icon name="trash-2" /></IconButton></span>
		</PropRow>
		<PropRow label={m['storyboard.pose']()}>
			<PropSelect label={m['storyboard.pose']()} options={optionsOf(POSES, f.pose)} value={f.pose} onchange={(v) => set(i, 'pose', v)} />
			<PropSelect label={m['storyboard.facing']()} options={optionsOf(FACING, f.facing)} value={f.facing} onchange={(v) => set(i, 'facing', v)} />
		</PropRow>
		<PropRow label={m['storyboard.framing']()}>
			<PropSelect label={m['storyboard.framing']()} options={optionsOf(SIZES, f.framing)} value={f.framing} onchange={(v) => set(i, 'framing', v)} />
			<PropSelect label={m['storyboard.mood']()} options={optionsOf(MOODS, f.mood)} placeholder="–" value={f.mood} onchange={(v) => set(i, 'mood', v)} />
		</PropRow>
		<PropRow label={m['storyboard.place']()} hint={m['storyboard.place_hint']()}>
			<ScrubNumber label="x" tag="X" step={0.01} value={f.x} onchange={(v) => set(i, 'x', v)} />
			<ScrubNumber label="y" tag="Y" step={0.01} value={f.y} onchange={(v) => set(i, 'y', v)} />
			<ScrubNumber label={m['storyboard.figure_height']()} tag="H" step={0.01} min={0} value={f.h} onchange={(v) => set(i, 'h', v)} />
		</PropRow>
	</div>
{/each}
<div class="add">
	<button type="button" class="secondary" title={m['storyboard.add_figure_hint']()} disabled={!castRefs.length} onclick={add} data-testid="add-figure"><Icon name="plus" /> {m['storyboard.add_figure']()}</button>
</div>

<style>
	.figure {
		display: grid;
		gap: var(--space-05);
		padding-top: var(--space-05);
		border-top: var(--border-width-regular) solid var(--border-subtle);
	}
	.figure:first-child {
		border-top: 0;
		padding-top: 0;
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
