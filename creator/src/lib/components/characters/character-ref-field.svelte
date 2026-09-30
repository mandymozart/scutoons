<script lang="ts" module>
	import type { CastEntry, CharacterRegistry, MoodVocabulary } from '@scutoons/content';
	import type { ToonInfo } from '$lib/editor/characters.ts';

	/** What the VN character fields pick from (see the chapter editor). */
	export interface CharacterChoices {
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		toons: ToonInfo[];
		/** the cast of the chapter's season */
		cast: CastEntry[];
		/** e.g. "Season 1" */
		season: string;
		locale: string;
		/** after a pick: adds the character / persona to the season's cast when it isn't there (with an undo notice) */
		ensureCast: (character: string, persona: string) => void;
	}
</script>

<script lang="ts">
	import { isKnownMissing, imageExists } from '$lib/editor/characters.ts';
	import {
		AUTHORED_VIEW,
		REGULAR_PERSONA,
		characterImage,
		characterIndex,
		characterName,
		findPersona,
		getAt,
		resolveCharacterId,
		lookLabel,
		moodLabel,
		type Path
	} from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CharacterPickerSheet, { type PickedCharacter } from './character-picker-sheet.svelte';

	/**
	 * A VN character reference: shows who (thumbnail, "Cat Pope · Biker · with helmet · happy") and opens the visual
	 * picker. `scene` refs (joinScene/leaveScene) store `character`, `persona`, `look?`, `mood?` (+ `variant`, the view);
	 * `speaker` refs (showMessage) store `character` and `persona`. Written to every locale file (one undo step).
	 */
	let {
		doc,
		path,
		kind = 'scene',
		label,
		choices,
		placeholder = '–'
	}: {
		doc: ContentDoc;
		/** the ref object (scene) or the event params (speaker) */
		path: Path;
		kind?: 'scene' | 'speaker';
		label: string;
		choices: CharacterChoices;
		placeholder?: string;
	} = $props();

	let open = $state(false);
	const ref = $derived<Record<string, any>>(getAt(doc.data[doc.primary], path) ?? {});
	// old refs ({ name }, old ids like plf) are read as their current form
	const normalized = $derived(resolveCharacterId(ref.character ?? ref.name ?? '', ref.persona));
	const id = $derived<string>(ref.character ?? ref.name ? normalized.character : '');
	const persona = $derived<string>(normalized.persona);
	const index = $derived(characterIndex(choices.registry));
	const character = $derived(index.get(id) ?? null);
	const inCast = $derived(choices.cast.find((e) => e.character === id));
	const src = $derived(character ? characterImage(character, { persona, look: ref.look, mood: ref.mood }, AUTHORED_VIEW, { isMissing: isKnownMissing }) : '');
	const summary = $derived.by(() => {
		if (!id) return placeholder;
		if (!character) return `${id} (${m['creator.characters_unknown']()})`;
		const parts = [characterName(character, choices.locale, persona)];
		if (kind === 'scene' && ref.look) parts.push(lookLabel(findPersona(character, persona), ref.look, choices.locale));
		if (kind === 'scene' && ref.mood) parts.push(moodLabel(ref.mood, choices.locale, choices.moods));
		return parts.join(' · ');
	});

	function pick(p: PickedCharacter) {
		doc.structural((file) => {
			let target = getAt(file, path);
			if (!target) return;
			// the current form: { character, persona, look?, mood?, variant? } (old `name` / `position` go)
			target.character = p.character;
			target.persona = p.persona || REGULAR_PERSONA;
			delete target.name;
			if (kind === 'scene') {
				if (target.position?.variant && !target.variant) target.variant = target.position.variant;
				delete target.position;
				const c = index.get(p.character);
				const defaultLook = findPersona(c, p.persona)?.looks.find((l) => l.default)?.id;
				if (p.look && p.look !== defaultLook) target.look = p.look;
				else delete target.look;
				if (p.mood && p.mood !== 'neutral') target.mood = p.mood;
				else delete target.mood;
			}
		});
		choices.ensureCast(p.character, p.persona || REGULAR_PERSONA);
	}
</script>

<div class="field ref-field" data-testid="character-ref">
	<span class="label">{label}</span>
	<button type="button" class="ref" class:unknown={!!id && !character} class:off-cast={!!character && (!inCast || !inCast.personas.includes(persona))} onclick={() => (open = true)} data-testid="character-ref-open">
		<span class="img" style:background={character?.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" />{/if}</span>
		<span class="text">{summary}</span>
		<span class="hint">{m['creator.characters_choose']()} …</span>
	</button>
	{#if character && (!inCast || !inCast.personas.includes(persona))}
		<button type="button" class="ghost warn" onclick={() => choices.ensureCast(id, persona)}>{m['creator.characters_add_to_cast']()}</button>
	{/if}
</div>

<CharacterPickerSheet
	bind:open
	registry={choices.registry}
	moods={choices.moods}
	toons={choices.toons}
	cast={choices.cast}
	season={choices.season}
	locale={choices.locale}
	mode={kind === 'scene' ? 'expression' : 'persona'}
	value={{ character: id, persona, look: ref.look, mood: ref.mood }}
	onpick={pick}
/>

<style>
	.ref-field {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}
	.label {
		font-size: 11px;
	}
	.ref {
		all: unset;
		box-sizing: border-box;
		cursor: pointer;
		display: flex;
		gap: 0.4rem;
		align-items: center;
		border: 1px solid var(--border-subtle);
		padding: 0.15rem 0.3rem;
		font-size: 12px;
		min-width: 0;
	}
	.ref:hover,
	.ref:focus-visible {
		border-color: var(--ink);
	}
	.ref.unknown,
	.ref.off-cast {
		border-style: dashed;
		border-color: var(--error);
	}
	.img {
		width: 1.8rem;
		height: 1.8rem;
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.img img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.warn {
		align-self: flex-start;
		font-size: 11px;
		color: var(--error);
	}
</style>
