<script lang="ts" module>
	import type { CastEntry, CharacterRegistry, MoodVocabulary } from '@scutoons/content';

	/** What the VN character fields show and check (see the chapter editor); the Library is what they pick from. */
	export interface CharacterChoices {
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		/** the cast of the chapter's season */
		cast: CastEntry[];
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
		findPersona,
		getAt,
		resolveCharacterId,
		lookLabel,
		moodLabel,
		type Path
	} from '@scutoons/content';
	import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { library, type PickedThing } from '$lib/editor/library.svelte.ts';
	import { thingId } from '$lib/editor/library-list.ts';

	/**
	 * A VN character reference: shows who (thumbnail, "Cat Pope · Biker · with helmet · happy") and opens the Library on
	 * characters – the season's cast first, a press steps into the character for its persona. `scene` refs
	 * (joinScene/leaveScene) store `character`, `persona`, `look?`, `mood?` (+ `variant`, the view); `speaker` refs
	 * (showMessage) store `character` and `persona`. Written to every locale file (one undo step).
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
		const parts = [shownCharacterName(character, persona)];
		if (kind === 'scene' && ref.look) parts.push(lookLabel(findPersona(character, persona), ref.look, choices.locale));
		if (kind === 'scene' && ref.mood) parts.push(moodLabel(ref.mood, choices.locale, choices.moods));
		return parts.join(' · ');
	});

	/** the slot asks the Library: characters, the season's cast first; a press steps into one for its persona */
	function open() {
		const type = getAt(doc.data[doc.primary], [...path.slice(0, -1), 'type']);
		library.startPick({
			label: typeof type === 'string' ? `${label} · ${type}` : label,
			types: ['character'],
			depth: 'persona',
			first: 'cast',
			value: id ? `character:${id}` : undefined,
			onthing: (picked) => pick(picked)
		});
	}

	function pick(picked: PickedThing) {
		const p = { ...picked, character: thingId(picked.address) };
		doc.edit((file) => {
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
	<button
		type="button"
		class="ref"
		class:unknown={!!id && !character}
		class:off-cast={!!character && (!inCast || !inCast.personas.includes(persona))}
		title={m['creator.characters_choose']()}
		onclick={open}
		data-testid="character-ref-open"
	>
		<span class="img" style:background={character?.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" />{/if}</span>
		<span class="text">{summary}</span>
	</button>
	{#if character && (!inCast || !inCast.personas.includes(persona))}
		<button type="button" class="ghost warn" onclick={() => choices.ensureCast(id, persona)}>{m['creator.characters_add_to_cast']()}</button>
	{/if}
</div>

<style>
	.ref-field {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}
	.label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--tool-text);
	}
	.ref {
		all: unset;
		box-sizing: border-box;
		cursor: pointer;
		display: flex;
		gap: 0.4rem;
		align-items: center;
		min-height: var(--control-height);
		border: var(--border-width-regular) solid var(--tool-text);
		padding: 0 0.3rem;
		min-width: 0;
	}
	.ref:hover,
	.ref:focus-visible {
		border-color: var(--tool-text);
	}
	.ref.unknown,
	.ref.off-cast {
		border-style: dashed;
		border-color: var(--tool-stop);
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
		font-size: var(--type-caption-size);
		color: var(--tool-stop);
	}
</style>
