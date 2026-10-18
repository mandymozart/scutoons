<script lang="ts">
	import { Dialogs } from '@scutoons/reader';
	import { defaultTail, type Dialog } from '@scutoons/content';
	import { TEXT_PRESETS, type TextPreset, type TextPresetId } from '$lib/editor/text-presets.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "+ Add text" (owner 2026-10-03): the presets, each previewed with a sample text on the current slice; a click adds
	 * a text with that look at the preset's place.
	 */
	let {
		image = '',
		ratio,
		lang,
		onpick
	}: {
		/** the slice's image (the preview's background) */
		image?: string;
		/** height / width of the slice */
		ratio: number;
		lang: string;
		onpick: (preset: TextPreset) => void;
	} = $props();

	const names: Record<TextPresetId, () => string> = {
		narration: m['creator.preset_narration'],
		speech: m['creator.preset_speech'],
		thought: m['creator.preset_thought'],
		onomatopoeia: m['creator.preset_onomatopoeia'],
		story: m['creator.preset_story'],
		scene: m['creator.preset_scene'],
		lyrics: m['creator.preset_lyrics']
	};
	const samples: Record<TextPresetId, () => string> = {
		narration: m['creator.preset_narration_sample'],
		speech: m['creator.preset_speech_sample'],
		thought: m['creator.preset_thought_sample'],
		onomatopoeia: m['creator.preset_onomatopoeia_sample'],
		story: m['creator.preset_story_sample'],
		scene: m['creator.preset_scene_sample'],
		lyrics: m['creator.preset_lyrics_sample']
	};
	/** the preview: speech / thought with their default tail */
	const sample = (p: TextPreset): Dialog[] => {
		const box = { left: p.rect.x, top: p.rect.y, width: p.rect.width ?? 80, height: p.rect.height ?? 12 };
		return [{ ...p.style, rect: p.rect, text: samples[p.id](), ...(p.tail ? { tail: defaultTail(p.tail, box, ratio) } : {}) }];
	};
</script>

<div class="presets" role="group" aria-label={m['creator.add_text_box']()} data-testid="text-presets">
	{#each TEXT_PRESETS as p (p.id)}
		<button type="button" class="preset" onclick={() => onpick(p)} data-testid="text-preset-{p.id}">
			<span class="preview" style:aspect-ratio="1 / {ratio}">
				{#if image}<img src={image} alt="" draggable="false" />{/if}
				<Dialogs dialogs={sample(p)} {lang} />
			</span>
			<span class="name">{names[p.id]()}</span>
		</button>
	{/each}
</div>

<style>
	.presets {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(7rem, 1fr));
		gap: var(--space-05);
	}
	.preset {
		all: unset;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		cursor: pointer;
		border: var(--border-width-regular) solid var(--border-muted);
		padding: 0.25rem;
	}
	.preset:hover,
	.preset:focus-visible {
		border-color: var(--ink);
	}
	/* a small slice: the reader's dialogs size in cqw of it */
	.preview {
		position: relative;
		display: block;
		width: 100%;
		overflow: hidden;
		container-type: inline-size;
		background: var(--border-subtle);
	}
	.preview img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: top;
		opacity: 0.6;
	}
	.name {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
</style>
