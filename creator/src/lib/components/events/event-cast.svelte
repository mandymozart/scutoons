<script lang="ts">
	import { tick } from 'svelte';
	import { CloseIcon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { viewSlots, type CastOptions } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import CharacterRefField, { type CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

	/**
	 * joinScene / leaveScene: one row per character (the cast picker, or a plain select without one); joining also sets
	 * the slot / view, the mood and – when the persona has more than one – the look.
	 */
	let {
		doc,
		path,
		event,
		index,
		choices,
		charOptions,
		cast
	}: {
		doc: ContentDoc;
		path: Path;
		event: any;
		index: number;
		choices?: CharacterChoices;
		charOptions: { value: string; label: string }[];
		cast: CastOptions;
	} = $props();

	const rows = $derived<any[]>(getAt(event, ['params', 'characters']) ?? []);
	const joining = $derived(event.type === 'joinScene');

	/** a new row, empty, its picker focused (owner 2026-10-03: the button "did not work" – it copied the first character) */
	async function addCharacter() {
		let row = 0;
		doc.edit((file) => {
			const params = getAt(file, [...path, 'params']);
			if (!params) return;
			params.characters ??= [];
			params.characters.push({ character: '', persona: 'regular', ...(getAt(file, [...path, 'type']) === 'joinScene' ? { variant: 'avatar' } : {}) });
			row = params.characters.length - 1;
		});
		await tick();
		document.querySelectorAll<HTMLElement>(`[data-testid="event-${index}"] [data-testid="character-ref-open"]`)[row]?.focus();
	}
	const removeCharacter = (ci: number) => doc.edit((file) => getAt(file, [...path, 'params', 'characters'])?.splice(ci, 1));
</script>

{#each rows as row, ci (ci)}
	{@const looks = cast.looksOf(row)}
	<div class="sc-fields cast-row" data-density="dense" data-align="end">
		{#if choices}
			<CharacterRefField {doc} path={[...path, 'params', 'characters', ci]} label={m['creator.characters_character']()} {choices} />
		{:else}
			<PropField {doc} path={[...path, 'params', 'characters', ci, 'name']} label={m['creator.characters_character']()} type="select" options={charOptions} required />
		{/if}
		{#if joining}
			<PropField {doc} path={[...path, 'params', 'characters', ci, row?.position && !row?.variant ? 'position' : 'variant', ...(row?.position && !row?.variant ? ['variant'] : [])]} label={m['creator.cast_slot']()} type="select" options={viewSlots()} placeholder={m['creator.view_slot_avatar']()} />
			<PropField {doc} path={[...path, 'params', 'characters', ci, 'mood']} label={m['creator.characters_mood']()} type="select" options={cast.moodOptions(row)} placeholder="neutral" />
			{#if looks.length > 1 || row?.look}
				<PropField {doc} path={[...path, 'params', 'characters', ci, 'look']} label={m['creator.characters_look']()} type="select" options={looks.slice(1).map((l) => ({ value: l.id, label: l.label }))} placeholder={looks[0] ? `${looks[0].label} (${m['creator.characters_default']()})` : m['creator.characters_default']()} />
			{/if}
		{/if}
		<button type="button" class="ghost" onclick={() => removeCharacter(ci)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
	</div>
{/each}
<button type="button" class="secondary" onclick={addCharacter}>+ {m['creator.cast_add_character']()}</button>

<style>
	/* character, slot, mood, remove (tools.css .sc-fields; only the columns are this row's own) */
	.cast-row {
		grid-template-columns: 2fr 1fr 1fr auto;
	}
</style>
