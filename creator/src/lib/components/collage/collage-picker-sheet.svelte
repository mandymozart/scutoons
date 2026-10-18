<script lang="ts">
	import { Tabs } from '@scutoons/ui';
	import { AssetPickerSheet, type PickedAsset } from '@scutoons/media';
	import { AUTHORED_VIEW, moodLabel, type CastEntry, type CharacterRegistry, type MoodVocabulary, type PieceFigure } from '@scutoons/content';
	import PickerSheet from '$lib/components/controls/picker-sheet.svelte';
	import CharacterGrid, { type PickedCharacter } from '$lib/components/characters/character-grid.svelte';
	import ShapeGrid from '$lib/components/collage/shape-grid.svelte';
	import { figureImage, imagePick, imageRatio, type PickedPiece } from '$lib/editor/collage-pick.ts';
	import { isKnownMissing, type ToonInfo } from '$lib/editor/characters.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The ONE picker of collage pieces (research/COLLAGE.md §5), for slices and scores: Shapes (the vector library) ·
	 * Characters (a character's mood portrait – the view is changed in the piece inspector); "Image …" hands over to the
	 * asset picker for any image. A pick hands back the file from the CDN root (`PickedPiece`) – the host writes it in
	 * its path form – and closes the sheet.
	 */
	let {
		open = $bindable(false),
		toon = '',
		registry,
		moods,
		cast = [],
		toons = [],
		onpick
	}: {
		open?: boolean;
		/** the toon the host belongs to (the asset picker opens on its folder; '' = every folder) */
		toon?: string;
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		/** shown first in the Characters tab (a pick never adds to the cast – a piece is art, not a role) */
		cast?: CastEntry[];
		toons?: ToonInfo[];
		onpick: (picked: PickedPiece) => void;
	} = $props();

	let tab = $state('shapes');
	let imageOpen = $state(false);

	function pick(picked: PickedPiece) {
		onpick(picked);
		open = false;
	}

	/** the sheet's openings: a decode that outlives its sheet (closed – Esc –, or closed and opened again) picks nothing */
	let opened = 0;
	$effect(() => {
		if (open) opened++;
	});

	/**
	 * a mood portrait: its file now (the fallback rule), its ratio by decoding it (a second click meanwhile adds nothing);
	 * a character without any picture adds nothing – a toast says so (the "?" substitute is no art)
	 */
	let decoding = false;
	async function pickCharacter(p: PickedCharacter) {
		const character = registry?.characters.find((c) => c.id === p.character);
		if (!character || decoding) return;
		const figure: PieceFigure = { id: p.character, view: AUTHORED_VIEW };
		if (p.persona) figure.persona = p.persona;
		if (p.look) figure.look = p.look;
		if (p.mood) figure.mood = p.mood;
		const src = figureImage(character, figure, isKnownMissing);
		if (!src) return void toast(m['creator.collage_no_picture']({ name: shownCharacterName(character, p.persona) || p.character }), { type: 'warning' });
		const name = [shownCharacterName(character, p.persona), p.mood && moodLabel(p.mood, textLanguage.names, moods)].filter(Boolean).join(' · ');
		decoding = true;
		const opening = opened;
		try {
			const ratio = await imageRatio(asset(src));
			if (!open || opened !== opening) return;
			pick({ src, ratio: ratio ?? undefined, figure, name });
		} finally {
			decoding = false;
		}
	}

	function pickImage(picked: PickedAsset) {
		const piece = imagePick(picked.asset, picked.rendition, picked.path);
		if (piece.ratio) return onpick(piece);
		void imageRatio(asset(piece.src)).then((ratio) => onpick({ ...piece, ratio: ratio ?? undefined }));
	}

	function toImages() {
		open = false;
		imageOpen = true;
	}
</script>

<PickerSheet bind:open title={m['creator.collage_picker_title']()} testid="collage-picker-sheet">
	<Tabs
		variant="ruled"
		label={m['creator.collage_picker_title']()}
		bind:value={tab}
		tabs={[
			{ id: 'shapes', label: m['creator.collage_shapes'](), testid: 'collage-tab-shapes' },
			{ id: 'characters', label: m['creator.collage_characters'](), testid: 'collage-tab-characters' }
		]}
	>
		{#snippet after()}
			<button type="button" class="secondary" onclick={toImages} data-testid="collage-image">{m['creator.collage_image']()}</button>
		{/snippet}
	</Tabs>
	{#if tab === 'shapes'}
		<ShapeGrid active={open} onpick={pick} />
	{:else}
		<CharacterGrid active={open} filters={false} {registry} {moods} {toons} {cast} locale={textLanguage.names} onpick={pickCharacter} />
	{/if}
</PickerSheet>

<AssetPickerSheet bind:open={imageOpen} {toon} kind="image" webp title={m['creator.collage_image_title']()} onpick={pickImage} />
