<script lang="ts" module>
	export type { PickedCharacter } from '$lib/components/characters/character-grid.svelte';
</script>

<script lang="ts">
	import type { CastEntry, CharacterRegistry, MoodVocabulary } from '@scutoons/content';
	import type { ToonInfo } from '$lib/editor/characters.ts';
	import PickerSheet from '$lib/components/controls/picker-sheet.svelte';
	import CharacterGrid, { type PickedCharacter } from '$lib/components/characters/character-grid.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The visual character picker (every character field): the picker sheet around the CharacterGrid (its two steps –
	 * the characters, then a character's personas → looks → moods). A pick closes the sheet. Escape closes.
	 */
	let {
		open = $bindable(false),
		registry,
		moods,
		toons = [],
		cast = [],
		season,
		value,
		locale,
		mode = 'expression',
		onpick
	}: {
		open?: boolean;
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		toons?: ToonInfo[];
		/** the season's cast (shown first; picking someone else adds them – the caller does that) */
		cast?: CastEntry[];
		/** e.g. "Season 1" (section title) */
		season?: string;
		value?: Partial<PickedCharacter>;
		locale: string;
		mode?: 'expression' | 'persona';
		onpick: (picked: PickedCharacter) => void;
	} = $props();

	function pick(picked: PickedCharacter) {
		onpick(picked);
		open = false;
	}
</script>

<PickerSheet bind:open title={m['creator.characters_picker_title']()} testid="character-picker-sheet">
	<CharacterGrid active={open} {registry} {moods} {toons} {cast} {season} {value} {locale} {mode} onpick={pick} />
</PickerSheet>
