<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { ASK_TEXT_LIMIT, ASK_TEXT_MAX_LENGTH, type Path } from '@scutoons/content';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import VariableField from '$lib/components/fields/variable-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import CharacterRefField, { type CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

	/**
	 * askText (agents/EXPRESSIONS.md §6): a question with one input line – who asks (or narration), where the answer goes
	 * (a story variable: this toon's, a global one, another toon's – or the reader's name), the texts per language.
	 */
	let {
		doc,
		path,
		event,
		choices,
		charOptions
	}: { doc: ContentDoc; path: Path; event: any; choices?: CharacterChoices; charOptions: { value: string; label: string }[] } = $props();

	const reader = $derived(event?.params?.scope === 'reader');
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	{#if choices}
		<CharacterRefField {doc} path={[...path, 'params']} kind="speaker" label={m['creator.characters_character']()} {choices} placeholder={m['creator.vn_narrator']()} />
	{:else}
		<PropField {doc} path={[...path, 'params', 'character']} label={m['creator.characters_character']()} type="select" options={charOptions} placeholder={m['creator.vn_narrator']()} />
	{/if}
	<!-- where the answer goes (owner 2026-10-06): this toon's / a global / another toon's variable, or the reader's name -->
	<VariableField {doc} path={[...path, 'params', 'variable']} scope={[...path, 'params', 'scope']} testid="ask-variable" />
	<PropField {doc} path={[...path, 'params', 'maxLength']} label={m['creator.ask_max_length']()} type="number" step={1} min={1} max={ASK_TEXT_LIMIT} placeholder={String(ASK_TEXT_MAX_LENGTH)} />
</div>
{#if reader}<p class="hint">{m['creator.ask_scope_reader_hint']()}</p>{/if}
<LocaleText {doc} path={[...path, 'params', 'text']} label={m['creator.ask_question']()} rows={2} expressions />
<LocaleText {doc} path={[...path, 'params', 'placeholder']} label={m['creator.ask_placeholder']()} multiline={false} />
<!-- the answer button (owner 2026-10-06): its text and what it stores are two fields; no text = no button, nothing
     stored = the variable stays unset (a later condition can tell) -->
<LocaleText {doc} path={[...path, 'params', 'answer']} label={m['creator.ask_answer']()} multiline={false} />
<LocaleText {doc} path={[...path, 'params', 'default']} label={m['creator.ask_default']()} multiline={false} />

<style>
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
</style>
