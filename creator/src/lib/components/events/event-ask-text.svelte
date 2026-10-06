<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { ASK_TEXT_LIMIT, ASK_TEXT_MAX_LENGTH, type Path } from '@scutoons/content';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import CharacterRefField, { type CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

	/**
	 * askText (agents/EXPRESSIONS.md §6): a question with one input line – who asks (or narration), where the answer goes
	 * (a story variable of this toon, or the reader's name everywhere), the texts per language (the header's language).
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
	<PropField
		{doc}
		path={[...path, 'params', 'scope']}
		label={m['creator.ask_scope']()}
		type="select"
		options={[{ value: 'reader', label: m['creator.ask_scope_reader']() }]}
		placeholder={m['creator.ask_scope_toon']()}
		testid="ask-scope"
	/>
	{#if !reader}<PropField {doc} path={[...path, 'params', 'variable']} label={m['creator.field_variable']()} required testid="ask-variable" />{/if}
	<PropField {doc} path={[...path, 'params', 'maxLength']} label={m['creator.ask_max_length']()} type="number" step={1} min={1} max={ASK_TEXT_LIMIT} placeholder={String(ASK_TEXT_MAX_LENGTH)} />
</div>
{#if reader}<p class="hint">{m['creator.ask_scope_reader_hint']()}</p>{/if}
<LocaleText {doc} path={[...path, 'params', 'text']} label={m['creator.ask_question']()} rows={2} />
<LocaleText {doc} path={[...path, 'params', 'placeholder']} label={m['creator.ask_placeholder']()} multiline={false} />
<LocaleText {doc} path={[...path, 'params', 'default']} label={m['creator.ask_default']()} multiline={false} />

<style>
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
</style>
