<script lang="ts" module>
	export type Level = 'character' | 'persona' | 'look';
	export type InspectorTab = 'profile' | 'model' | 'wanted' | 'prompts';
</script>

<script lang="ts">
	import { FileSlot, ModelPreview } from '@scutoons/media';
	import { characterImage, personaModelSource, type AssetRequests, type CharacterPersona, type CharacterRegistry, type CreditPerson, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { cleanText } from '$lib/editor/character-form.ts';
	import { isKnownMissing, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { LookEdits } from '$lib/editor/look-edit.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import CharacterIdentity from '$lib/components/characters/character-identity.svelte';
	import CharacterPersonaFields from '$lib/components/characters/character-persona.svelte';
	import CharacterTransitions from '$lib/components/characters/character-transitions.svelte';
	import PersonaModel from '$lib/components/characters/persona-model.svelte';
	import AposeEditor from '$lib/components/characters/apose-editor.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import CharacterPrompts from '$lib/components/characters/character-prompts.svelte';

	/**
	 * The character page's inspector (research/CHARACTER-PAGE.md E, K3 – owner 2026-10-20: "wenn die anderen dinge im
	 * inspector auch so contextualisiert sind"): every tab shows the level picked on the page – the character (its head),
	 * a persona or a look – under one title (face · tab · level · name). Profile: the character's identity / the persona's
	 * fields + its transitions / the look's label, default, remove. Model: the persona's GLB + body / the look's own GLB
	 * and its A-pose. Wanted images: what this level lacks. Prompts: for a generator or an artist, from the persona + look.
	 * The tab row is the page's (desktop: above the panel; phones: the bar opens the sheet).
	 */
	let {
		tab,
		level,
		registry,
		file,
		moods,
		requests,
		character,
		persona,
		lookId,
		languages,
		source,
		toons,
		people,
		edits,
		bust,
		personaName,
		lookName,
		onpersonaremoved,
		onlookremoved
	}: {
		tab: InspectorTab;
		level: Level;
		registry: ContentDoc;
		file: CharacterRegistry | null;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
		languages: string[];
		source: string;
		toons: ToonInfo[];
		people: CreditPerson[];
		edits: LookEdits;
		bust: number;
		personaName: string;
		lookName: string;
		onpersonaremoved: () => void;
		onlookremoved: () => void;
	} = $props();

	const TAB_LABELS: Record<InspectorTab, () => string> = {
		profile: () => m['creator.characters_tab_profile'](),
		model: () => m['creator.characters_tab_model'](),
		wanted: () => m['creator.characters_wanted'](),
		prompts: () => m['creator.prompts_title']()
	};
	const LEVEL_LABELS: Record<Level, () => string> = {
		character: () => m['creator.characters_character'](),
		persona: () => m['creator.characters_level_persona'](),
		look: () => m['creator.characters_level_look']()
	};
	const look = $derived(persona.looks.find((l) => l.id === lookId));
	const face = $derived(characterImage(character, level === 'character' ? {} : { persona: persona.id, look: level === 'look' ? lookId : undefined }, 'avatar', { isMissing: isKnownMissing }));
	const name = $derived(level === 'character' ? personaName.split(' · ')[0] : level === 'persona' ? personaName : lookName);
	const model = $derived(personaModelSource(character, persona.id, lookId));

	async function removePersona() {
		if (!(await confirmRemove(m['creator.characters_remove_persona_confirm']({ persona: personaName })))) return;
		onpersonaremoved();
	}
	async function removeLook() {
		if (persona.looks.length < 2 || !(await confirmRemove(m['creator.characters_remove_look_confirm']({ look: lookId })))) return;
		edits.remove();
		onlookremoved();
	}
</script>

<div class="inspector" data-testid="character-inspector" data-tab={tab} data-level={level}>
	<header class="title">
		{#if face}<img src={asset(face)} alt="" />{/if}
		<span class="t">
			<span class="cap">{TAB_LABELS[tab]()} · {LEVEL_LABELS[level]()}</span>
			<span class="name" data-testid="inspector-name">{name}</span>
		</span>
	</header>
	<div class="body">
		{#if tab === 'profile'}
			{#if level === 'character'}
				<CharacterIdentity {registry} {character} {languages} {toons} {people} />
			{:else if level === 'persona'}
				{#if persona.id === 'regular'}
					<p class="sc-empty">{m['creator.characters_regular_hint']()}</p>
				{:else}
					<CharacterPersonaFields {registry} {character} {persona} {languages} {toons} onremove={removePersona} />
				{/if}
				<CharacterTransitions {registry} characterId={character.id} {persona} {languages} />
			{:else if look}
				<LocaleMapField value={look.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => edits.setLabel(cleanText(v))} />
				<label class="check"><input type="radio" name="default-look-{character.id}-{persona.id}" checked={!!look.default} onchange={() => edits.setDefault()} /> {m['creator.characters_default_look']()}</label>
				{#if persona.looks.length > 1}<button type="button" class="ghost danger" onclick={removeLook} data-testid="remove-look">{m['creator.characters_remove_look']()}</button>{/if}
			{/if}
		{:else if tab === 'model'}
			{#if level === 'look' && look}
				<ModelPreview path={model.path} fallback={model.fallback} testid="look-model-preview" />
				<FileSlot kind="model" value={look.model} label={m['creator.characters_look_model']()} place="characters/{character.id}/" player={false} onchange={(p) => edits.setModel(p)} testid="look-model" />
				{#key `${persona.id}/${lookId}`}<AposeEditor character={character.id} persona={persona.id} {look} {bust} onchange={(p) => edits.setApose(p)} />{/key}
			{:else if level === 'persona'}
				<PersonaModel {registry} {character} {persona} {lookId} />
			{:else}
				<p class="sc-empty">{m['creator.characters_model_pick_level']()}</p>
			{/if}
		{:else if tab === 'wanted'}
			<WantedImages registry={file} {moods} {requests} locale={source} character={character.id} persona={level === 'character' ? undefined : persona.id} look={level === 'look' ? lookId : undefined} />
		{:else}
			<CharacterPrompts registry={file} {character} persona={persona.id} look={lookId} {moods} {requests} locale={source} />
		{/if}
	</div>
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		min-height: 0;
		flex: 1;
	}
	.title {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		flex: none;
	}
	.title img {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: cover;
		flex: none;
	}
	.t {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.cap {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: var(--space-1);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	/* one column of fields in the narrow panel */
	.body :global(.sc-fields) {
		--fields-fill-min: 100%;
		--fields-fit-min: 100%;
	}
	.check {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.danger {
		color: var(--state-error);
		align-self: flex-start;
	}
</style>
