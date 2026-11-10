<script lang="ts">
	import { tick } from 'svelte';
	import { CloseIcon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { AUTHORED_VIEW, characterImage, characterIndex, getAt, resolveCharacterId, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { REGULAR_PERSONA } from '@scutoons/content';
	import { viewSlots, type CastOptions } from '$lib/editor/vn-forms.ts';
	import { TARGETS } from '$lib/editor/address-targets.ts';
	import { imageExists, isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';

	/**
	 * joinScene / leaveScene: one row per character – who (character, persona, look) is ONE typeahead over the toon's
	 * address book (#477); joining also sets the slot / view and the mood. In the field's row (slice D): the character's
	 * thumbnail, and a warning with "add to the cast" when the persona is not in the season's cast; an old row `{ name }`
	 * shows its name.
	 */
	let {
		doc,
		path,
		event,
		index,
		choices,
		cast
	}: {
		doc: ContentDoc;
		path: Path;
		event: any;
		index: number;
		choices?: CharacterChoices;
		cast: CastOptions;
	} = $props();

	/** a picked character replaces an old `name` ref; one off the season's cast is added to it (with an undo notice) */
	function picked(t: any) {
		delete t.name;
		const character = t.character;
		const persona = t.persona || REGULAR_PERSONA;
		if (character) queueMicrotask(() => choices?.ensureCast(character, persona));
	}

	const rows = $derived<any[]>(getAt(event, ['params', 'characters']) ?? []);
	const joining = $derived(event.type === 'joinScene');
	const registry = $derived(characterIndex(choices?.registry ?? null));

	/** who a row shows: old ids (plf) and an old `name` read as their current form, as the cast picker did */
	function who(row: any) {
		const ref = resolveCharacterId(row?.character || row?.name || '', row?.persona);
		const character = registry.get(ref.character) ?? null;
		const src = character ? characterImage(character, { persona: ref.persona, look: row?.look, mood: row?.mood }, AUTHORED_VIEW, { isMissing: isKnownMissing }) : '';
		const entry = choices?.cast.find((e) => e.character === ref.character);
		return { ...ref, color: character?.color, src, offCast: !!character && !!choices && !entry?.personas.includes(ref.persona) };
	}

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
		document.querySelectorAll<HTMLElement>(`[data-testid="event-${index}"] [data-testid="cast-character-input"]`)[row]?.focus();
	}
	const removeCharacter = (ci: number) => doc.edit((file) => getAt(file, [...path, 'params', 'characters'])?.splice(ci, 1));
</script>

{#each rows as row, ci (ci)}
	{@const w = who(row)}
	<div class="sc-fields cast-row" class:leaving={!joining} data-density="dense" data-align="end">
		<AddressField {doc} path={[...path, 'params', 'characters', ci]} spec={TARGETS.cast} legacy="name" label={m['creator.characters_character']()} onpick={picked} testid="cast-character">
			{#snippet lead()}
				{#if w.color || w.src}<span class="thumb" style:background={w.color || 'transparent'}>{#if w.src}<img src={asset(w.src)} onerror={() => imageExists(w.src)} alt="" />{/if}</span>{/if}
			{/snippet}
			{#if w.offCast}
				<button type="button" class="ghost warn" onclick={() => choices?.ensureCast(w.character, w.persona)} data-testid="cast-off-cast">{m['creator.characters_add_to_cast']()}</button>
			{/if}
		</AddressField>
		{#if joining}
			<PropField {doc} path={[...path, 'params', 'characters', ci, row?.position && !row?.variant ? 'position' : 'variant', ...(row?.position && !row?.variant ? ['variant'] : [])]} label={m['creator.cast_slot']()} type="select" options={viewSlots()} placeholder={m['creator.view_slot_avatar']()} />
			<PropField {doc} path={[...path, 'params', 'characters', ci, 'mood']} label={m['creator.characters_mood']()} type="select" options={cast.moodOptions(row)} placeholder="neutral" />
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
	.cast-row.leaving {
		grid-template-columns: 1fr auto;
	}
	/* the character's thumbnail in the field's row (its colour behind it, as in the cast picker) */
	.thumb {
		flex: none;
		width: 1.8rem;
		height: 1.8rem;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.thumb img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.warn {
		align-self: flex-start;
		font-size: var(--type-caption-size);
		color: var(--error);
	}
</style>
