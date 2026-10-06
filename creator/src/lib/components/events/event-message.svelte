<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import type { CastOptions } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import CharacterRefField, { type CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

	/** showMessage: who speaks (the cast picker, or a plain select without one), wait for a tap, mood, the words */
	let {
		doc,
		path,
		choices,
		charOptions,
		cast
	}: { doc: ContentDoc; path: Path; choices?: CharacterChoices; charOptions: { value: string; label: string }[]; cast: CastOptions } = $props();
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	{#if choices}
		<CharacterRefField {doc} path={[...path, 'params']} kind="speaker" label={m['creator.characters_character']()} {choices} placeholder={m['creator.vn_narrator']()} />
	{:else}
		<PropField {doc} path={[...path, 'params', 'character']} label={m['creator.characters_character']()} type="select" options={charOptions} placeholder={m['creator.vn_narrator']()} />
	{/if}
	<PropField {doc} path={[...path, 'params', 'wait']} label={m['creator.vn_wait']()} type="bool" placeholder="false" />
	<PropField {doc} path={[...path, 'params', 'mood']} label={m['creator.characters_mood']()} type="select" options={cast.moodOptions(null)} placeholder="–" />
</div>
<LocaleText {doc} path={[...path, 'params', 'text']} rows={2} />
