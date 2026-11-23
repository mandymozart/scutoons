<script lang="ts" module>
	export type Level = 'character' | 'persona' | 'look';
</script>

<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { Section, Toggle } from '@scutoons/ui/editor';
	import { characterImage, lookLabel, lookPortraitMoods, pickText, REGULAR_PERSONA, type CharacterPersona, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { cleanText } from '$lib/editor/character-form.ts';
	import { isKnownMissing, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { KanonState } from '$lib/editor/kanon-docs.svelte.ts';
	import type { LookEdits } from '$lib/editor/look-edit.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import FaceRow from '$lib/components/characters/face-row.svelte';
	import KanonSquare from '$lib/components/characters/kanon-square.svelte';
	import CharacterIdentity from '$lib/components/characters/character-identity.svelte';
	import CharacterPersonaFields from '$lib/components/characters/character-persona.svelte';
	import CharacterTransitions from '$lib/components/characters/character-transitions.svelte';

	/**
	 * Who the character is, at the top of its Kanon tab: the character, its persona and its look as Sections, one under
	 * the other (no level to pick first). Each level's Kanon square opens that level's text under them.
	 */
	let {
		registry,
		moods,
		character,
		persona,
		lookId,
		languages,
		source,
		toons,
		edits,
		kanon,
		onpersona,
		onlook,
		onnewpersona,
		onnewlook,
		onkanon,
		onpersonaremoved,
		onlookremoved,
		onremove
	}: {
		registry: ContentDoc;
		moods: MoodVocabulary;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
		languages: string[];
		source: string;
		toons: ToonInfo[];
		edits: LookEdits;
		/** each level's Kanon state, and which one the Kanon tab shows */
		kanon: (level: Level) => { state: KanonState; open: boolean };
		onpersona: (id: string) => void;
		onlook: (id: string) => void;
		onnewpersona: (label: string) => void;
		onnewlook: (label: string) => void;
		onkanon: (level: Level) => void;
		onpersonaremoved: () => void;
		onlookremoved: () => void;
		onremove: () => void;
	} = $props();

	const look = $derived(persona.looks.find((l) => l.id === lookId));
	const face = (ref: { persona?: string; look?: string }) => characterImage(character, ref, 'avatar', { isMissing: isKnownMissing });
	const personaLabel = (p: CharacterPersona) => (p.id === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(p.label, textLanguage.names, [source, ...languages]) || p.id);
	const lookName = (id: string) => lookLabel(persona, id, textLanguage.names);

	async function removePersona() {
		if (!(await confirmRemove(m['creator.characters_remove_persona_confirm']({ persona: personaLabel(persona) })))) return;
		onpersonaremoved();
	}
	async function removeLook() {
		if (persona.looks.length < 2 || !(await confirmRemove(m['creator.characters_remove_look_confirm']({ look: lookId })))) return;
		edits.remove();
		onlookremoved();
	}
</script>

{#snippet kanonSquare(level: Level)}
	{@const k = kanon(level)}
	<KanonSquare state={k.state} open={k.open} label={m['creator.kanon']()} onclick={() => onkanon(level)} testid="kanon-{level}" />
{/snippet}

<div class="inspector" data-testid="character-inspector">
	<div class="who">
		<span class="avatar" style:background={character.color || null}>{#if face({})}<img src={asset(face({}))} alt="" />{/if}</span>
		<span class="t">
			<span class="name" data-testid="character-title">{shownCharacterName(character)}</span>
			<span class="id">{character.id}</span>
		</span>
		<button type="button" class="ib" onclick={onremove} title={m['creator.characters_remove']()} aria-label={m['creator.characters_remove']()} data-testid="remove-character"><Icon name="trash-2" /></button>
	</div>

	<Section id="character-identity" label={m['creator.characters_character']()} summary={character.id}>
		{#snippet aside()}{@render kanonSquare('character')}{/snippet}
		<CharacterIdentity {registry} {character} {languages} {toons} />
	</Section>

	<Section id="character-persona" label={m['creator.characters_level_persona']()} summary={personaLabel(persona)}>
		{#snippet aside()}{@render kanonSquare('persona')}{/snippet}
		<FaceRow
			label={m['creator.characters_personas']()}
			items={character.personas.map((p) => ({ id: p.id, label: personaLabel(p), src: face({ persona: p.id }) }))}
			value={persona.id}
			addLabel={m['creator.characters_persona_label_placeholder']()}
			onpick={onpersona}
			onadd={onnewpersona}
			testid="persona"
		/>
		{#if persona.id === REGULAR_PERSONA}
			<p class="hint">{m['creator.characters_regular_hint']()}</p>
		{:else}
			<CharacterPersonaFields {registry} {character} {persona} {languages} onremove={removePersona} />
		{/if}
		<CharacterTransitions {registry} characterId={character.id} {persona} {languages} />
	</Section>

	<Section id="character-look" label={m['creator.characters_level_look']()} summary="{lookName(lookId)} · {look ? lookPortraitMoods(look).length : 0} / {moods.moods.length}">
		{#snippet aside()}{@render kanonSquare('look')}{/snippet}
		<FaceRow
			label={m['creator.characters_looks']()}
			items={persona.looks.map((l) => ({ id: l.id, label: lookName(l.id), src: face({ persona: persona.id, look: l.id }), count: String(lookPortraitMoods(l).length), warning: lookPortraitMoods(l).length === 0 }))}
			value={lookId}
			addLabel={m['creator.characters_new_look']()}
			onpick={onlook}
			onadd={onnewlook}
			testid="look"
		/>
		{#if look}
			<LocaleMapField value={look.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => edits.setLabel(cleanText(v))} />
			<div class="row">
				<span>{m['creator.characters_default_look']()}</span>
				<Toggle on={!!look.default} label={m['creator.characters_default_look']()} disabled={!!look.default} onchange={(on) => on && edits.setDefault()} />
			</div>
			{#if persona.looks.length > 1}<button type="button" class="ghost danger" onclick={removeLook} data-testid="remove-look">{m['creator.characters_remove_look']()}</button>{/if}
		{/if}
	</Section>
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding-bottom: var(--editor-pad);
	}
	.who {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0 var(--editor-pad) 0.75rem;
		min-width: 0;
	}
	.avatar {
		flex: none;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--editor-radius);
		overflow: hidden;
		background: var(--tool-control);
	}
	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
	}
	.name,
	.id {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name {
		color: var(--tool-text);
	}
	.id {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.ib {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		border-radius: 50%;
		color: var(--tool-muted);
		cursor: pointer;
	}
	.ib:hover,
	.ib:focus-visible {
		background: var(--tool-hover);
		color: var(--tool-text);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.hint {
		margin: 0;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	/* one column of fields in the narrow panel */
	.inspector :global(.sc-fields) {
		--fields-fill-min: 100%;
		--fields-fit-min: 100%;
	}
	.danger {
		color: var(--tool-danger);
		justify-self: start;
	}
</style>
