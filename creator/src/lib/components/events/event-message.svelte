<script lang="ts">
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
		<CharacterRefField {doc} path={[...path, 'params']} kind="speaker" label="character" {choices} placeholder="narrator" />
	{:else}
		<PropField {doc} path={[...path, 'params', 'character']} label="character" type="select" options={charOptions} placeholder="narrator" />
	{/if}
	<PropField {doc} path={[...path, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
	<PropField {doc} path={[...path, 'params', 'mood']} label="mood" type="select" options={cast.moodOptions(null)} placeholder="–" />
</div>
<LocaleText {doc} path={[...path, 'params', 'text']} rows={2} />
