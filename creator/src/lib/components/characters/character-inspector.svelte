<script lang="ts" module>
	export type Level = 'character' | 'persona' | 'look';
</script>

<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { Section, Toggle } from '@scutoons/ui/editor';
	import { FileSlot, ModelPreview } from '@scutoons/media';
	import { castSeasons, characterImage, lookLabel, lookPortraitMoods, personaModelSource, pickText, REGULAR_PERSONA, type AssetRequests, type CharacterPersona, type CharacterRegistry, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { cleanText } from '$lib/editor/character-form.ts';
	import { castOf, isKnownMissing, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editHref } from '$lib/editor/edit-key.ts';
	import type { KanonState } from '$lib/editor/kanon-docs.svelte.ts';
	import type { LookEdits } from '$lib/editor/look-edit.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import FaceRow from '$lib/components/characters/face-row.svelte';
	import KanonSquare from '$lib/components/characters/kanon-square.svelte';
	import CharacterImport from '$lib/components/characters/character-import.svelte';
	import CharacterIdentity from '$lib/components/characters/character-identity.svelte';
	import CharacterPersonaFields from '$lib/components/characters/character-persona.svelte';
	import CharacterTransitions from '$lib/components/characters/character-transitions.svelte';
	import CharacterLinks from '$lib/components/characters/character-links.svelte';
	import PersonaModel from '$lib/components/characters/persona-model.svelte';
	import AposeEditor from '$lib/components/characters/apose-editor.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import CharacterPrompts from '$lib/components/characters/character-prompts.svelte';

	/**
	 * The character workspace's Inspector: the character, its persona and its look as Sections, one under the other (no
	 * level to pick first), then where it is cast (Links), its prompts and its wanted images. Each level's Kanon square
	 * opens that level's text in the Kanon tab.
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		file,
		moods,
		requests,
		character,
		persona,
		lookId,
		languages,
		source,
		toons,
		casts,
		edits,
		bust = $bindable(0),
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
		moodsDoc: ContentDoc;
		requestsDoc: ContentDoc;
		file: CharacterRegistry | null;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
		languages: string[];
		source: string;
		toons: ToonInfo[];
		/** the toons' cast documents (saved with the page) */
		casts: Record<string, ContentDoc>;
		edits: LookEdits;
		bust?: number;
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
	const model = $derived(personaModelSource(character, persona.id, lookId));
	const face = (ref: { persona?: string; look?: string }) => characterImage(character, ref, 'avatar', { isMissing: isKnownMissing });
	const personaLabel = (p: CharacterPersona) => (p.id === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(p.label, textLanguage.names, [source, ...languages]) || p.id);
	const lookName = (id: string) => lookLabel(persona, id, textLanguage.names);
	/** how many seasons it is cast in */
	const castIn = $derived(
		Object.values(casts).reduce((n, doc) => {
			const cast = castOf(doc);
			return n + castSeasons(cast).filter((s) => cast?.seasons[String(s)]?.some((e) => e.character === character.id)).length;
		}, 0)
	);

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
		<CharacterImport {registry} {moodsDoc} {requestsDoc} {moods} {requests} {character} {languages} bind:bust compact />
		<a class="ib" href={editHref(`rig:character.${character.id}`, { w: 'sound' })} title={m['creator.characters_sound_design']()} aria-label={m['creator.characters_sound_design']()} data-testid="character-sound-design"><Icon name="music" /></a>
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
		<Section id="character-persona-model" label={m['creator.characters_tab_model']()} nested open={false}>
			<PersonaModel {registry} {character} {persona} {lookId} />
		</Section>
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
			<Section id="character-look-model" label={m['creator.characters_tab_model']()} nested open={false} summary={look.model ? look.model.split('/').at(-1) : ''}>
				<ModelPreview path={model.path} fallback={model.fallback} testid="look-model-preview" />
				<FileSlot kind="model" value={look.model} label={m['creator.characters_look_model']()} place="characters/{character.id}/" player={false} onchange={(p) => edits.setModel(p)} testid="look-model" />
				{#key `${persona.id}/${lookId}`}<AposeEditor character={character.id} persona={persona.id} {look} {bust} onchange={(p) => edits.setApose(p)} />{/key}
			</Section>
			{#if persona.looks.length > 1}<button type="button" class="ghost danger" onclick={removeLook} data-testid="remove-look">{m['creator.characters_remove_look']()}</button>{/if}
		{/if}
	</Section>

	<Section id="character-links" label={m['creator.characters_links']()} summary={m['creator.characters_links_summary']({ count: castIn })}>
		<CharacterLinks {character} {registry} {casts} {toons} />
	</Section>

	<Section id="character-prompts" label={m['creator.prompts_title']()} open={false}>
		<CharacterPrompts registry={file} {character} persona={persona.id} look={lookId} {moods} {requests} locale={source} />
	</Section>

	<Section id="character-wanted" label={m['creator.characters_wanted']()} open={false}>
		<WantedImages registry={file} {moods} {requests} locale={source} character={character.id} persona={persona.id} look={lookId} />
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
		background: var(--editor-control);
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
		color: var(--editor-title, var(--editor-text));
	}
	.id {
		color: var(--editor-muted);
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
		color: var(--editor-muted);
		cursor: pointer;
	}
	.ib:hover,
	.ib:focus-visible {
		background: var(--editor-hover);
		color: var(--editor-text);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.hint {
		margin: 0;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	/* one column of fields in the narrow panel */
	.inspector :global(.sc-fields) {
		--fields-fill-min: 100%;
		--fields-fit-min: 100%;
	}
	.danger {
		color: var(--editor-danger);
		justify-self: start;
	}
</style>
