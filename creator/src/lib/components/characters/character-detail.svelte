<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { Badge, Icon, Tabs } from '@scutoons/ui';
	import {
		REGULAR_PERSONA,
		castsOf,
		defaultLookId,
		findPersona,
		isRetiredAposeLook,
		lookLabel,
		lookPortraitMoods,
		personaIds,
		pickText,
		type CharacterRegistry,
		type CreditPerson,
		type MoodVocabulary,
		type RegistryCharacter,
		type ToonCast
	} from '@scutoons/content';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { addLook, addPersona, characterLanguages, edit, editCharacter, editPersona, registryOf, requestsOf, type ToonInfo } from '$lib/editor/characters.ts';
	import { FileSlot } from '@scutoons/media';
	import { aposeLooksOf, applyApose, closeRequests } from '$lib/editor/character-form.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SourceMark from '$lib/components/text/source-mark.svelte';
	import CharacterIdentity from '$lib/components/characters/character-identity.svelte';
	import CharacterImport from '$lib/components/characters/character-import.svelte';
	import CharacterPersonaFields from '$lib/components/characters/character-persona.svelte';
	import CharacterLook from '$lib/components/characters/character-look.svelte';
	import CharacterTransitions from '$lib/components/characters/character-transitions.svelte';
	import AposeEditor from '$lib/components/characters/apose-editor.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import CharacterPrompts from '$lib/components/characters/character-prompts.svelte';

	/**
	 * One character (COMPONENTS.md step 9: in sections): identity (CharacterIdentity), the image import, its personas as
	 * tabs → the persona's own fields (CharacterPersonaFields) → its looks as sub-tabs → the look's mood grid
	 * (CharacterLook) and transitions, the persona's 3D model (a look may have its own – owner 2026-10-13), the A-pose looks, the character's wanted images and – at the very end – prompts
	 * for an image generator or an artist (CharacterPrompts).
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		moods,
		id,
		persona = $bindable(REGULAR_PERSONA),
		look = $bindable(''),
		toons,
		casts,
		people,
		onselect
	}: {
		registry: ContentDoc;
		moodsDoc: ContentDoc;
		requestsDoc: ContentDoc;
		moods: MoodVocabulary;
		id: string;
		persona?: string;
		look?: string;
		toons: ToonInfo[];
		casts: Record<string, ToonCast | null>;
		people: CreditPerson[];
		onselect: (id: string) => void;
	} = $props();

	const file = $derived(registryOf(registry));
	const character = $derived(file?.characters.find((c) => c.id === id) ?? null);
	const appearsIn = $derived(character ? castsOf(character.id, casts) : []);
	const langs = $derived(characterLanguages(character, toons, appearsIn));
	const languages = $derived(langs.languages as string[]);
	const source = $derived(langs.source as string);
	const personas = $derived(personaIds(character));
	const personaId = $derived(personas.includes(persona) ? persona : REGULAR_PERSONA);
	const currentPersona = $derived(findPersona(character, personaId));
	const lookIds = $derived((currentPersona?.looks ?? []).map((l) => l.id));
	const lookId = $derived(lookIds.includes(look) ? look : defaultLookId(currentPersona));
	const requests = $derived(requestsOf(requestsDoc));
	let newPersonaLabel = $state('');
	let newLookLabel = $state('');
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);

	function change(fn: (c: RegistryCharacter) => void, key?: string) {
		editCharacter(registry, id, fn, key ? `${id}:${key}` : undefined);
	}
	/** also the phone bar's Delete (characters-editor.svelte, the selection's row) */
	export async function remove() {
		if (!character) return;
		const used = appearsIn.length ? m['creator.characters_remove_used']({ toons: appearsIn.join(', ') }) : '';
		if (!(await confirmRemove(`${m['creator.characters_remove_confirm']({ name: shownCharacterName(character) })} ${used}`.trim()))) return;
		edit<CharacterRegistry>(registry, (f) => (f.characters = f.characters.filter((c) => c.id !== id)));
		onselect('');
	}
	function newPersona() {
		if (!newPersonaLabel.trim()) return;
		persona = addPersona(registry, id, newPersonaLabel, languages);
		look = '';
		newPersonaLabel = '';
	}
	async function removePersona() {
		if (personaId === REGULAR_PERSONA || !(await confirmRemove(m['creator.characters_remove_persona_confirm']({ persona: shownCharacterName(character, personaId) })))) return;
		change((c) => (c.personas = c.personas.filter((p) => p.id !== personaId)));
		persona = REGULAR_PERSONA;
	}
	/** a persona's 3D model (a .glb from the asset manager); cleared = none */
	function setPersonaModel(path: string | null) {
		editPersona(registry, id, personaId, (p) => (path ? (p.model = path) : delete p.model), `${id}:${personaId}:model`);
	}
	function newLook() {
		if (!newLookLabel.trim()) return;
		look = addLook(registry, id, personaId, newLookLabel, languages);
		newLookLabel = '';
	}

	/** the A-pose looks shown: the stored ones plus the standard looks not stored yet (empty slots) */
	const aposeLooks = $derived(aposeLooksOf(character));
	/**
	 * an A-pose slot picked (a path closes its open request) or cleared (an unused standard look disappears again, a
	 * retired one – the removed naked look – always)
	 */
	function setAposePath(target: string, path: string | undefined) {
		change((c) => {
			if (path) return applyApose(c, target, path, languages);
			const looks = c.apose?.looks;
			const look = looks?.find((l) => l.id === target);
			if (!looks || !look) return;
			delete look.path;
			if (isRetiredAposeLook(look) || (!look.slots?.length && aposeLooksOf(null).some((l) => l.id === target))) looks.splice(looks.indexOf(look), 1);
			if (!looks.length) delete c.apose;
			else if (!looks.some((l) => l.default)) looks[0].default = true;
		}, `apose-${target}-path`);
		if (path) closeRequests(requestsDoc, requests, id, [{ kind: 'apose', look: target, path }]);
	}
</script>


{#if character}
	<article class="detail" data-testid="character-detail" data-id={character.id}>
		<header class="head">
			<div>
				<span class="type">{m['creator.characters_character']()}</span>
				<h2 data-testid="character-title">{shownCharacterName(character)}</h2>
				<span class="hint">id <code>{character.id}</code> · {m['creator.characters_languages']()} {#each languages as l, i (l)}{i ? ', ' : ''}{l}{#if l === source}<SourceMark />{/if}{/each}</span>
			</div>
			<button type="button" class="ghost danger" onclick={remove} data-testid="remove-character">{m['creator.characters_remove']()}</button>
		</header>
		<p class="relation" data-testid="appears-in">
			<span>{m['creator.characters_appears_in']()}:</span>
			{#each appearsIn as t (t)}<a href="/{t}/cast">{toons.find((x) => x.id === t)?.name ?? t}</a>{:else}<span class="hint">{m['creator.characters_in_no_cast']()}</span>{/each}
		</p>

		<CharacterIdentity {registry} {character} {languages} {toons} {people} />

		<!-- Personas → looks → moods -->
		<section class="block" data-testid="personas">
			<CharacterImport {registry} {moodsDoc} {requestsDoc} {moods} {requests} {character} {languages} bind:bust />
			<Tabs
				variant="ruled"
				label={m['creator.characters_personas']()}
				value={personaId}
				onchange={(p) => ((persona = p), (look = ''))}
				tabs={personas.map((p) => ({ id: p, label: p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, [source, ...languages]) || p, testid: `persona-${p}` }))}
			>
				{#snippet after()}
					<span class="add">
						<input type="text" bind:value={newPersonaLabel} placeholder={m['creator.characters_persona_label_placeholder']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), newPersona())} data-testid="new-persona" />
						<button type="button" class="ghost" onclick={newPersona} disabled={!newPersonaLabel.trim()} data-testid="add-persona"><Icon name="plus" /> {m['creator.characters_add_persona']()}</button>
					</span>
				{/snippet}
			</Tabs>
			{#if currentPersona}
				<div class="persona" data-testid="persona-panel" data-persona={personaId}>
					{#if personaId !== REGULAR_PERSONA}
						<CharacterPersonaFields {registry} {character} persona={currentPersona} {languages} {toons} onremove={removePersona} />
					{/if}
					<div class="model" title={m['creator.characters_model_hint']()}>
						<FileSlot kind="model" value={currentPersona.model} label={m['creator.characters_model']()} place="characters/{id}/" player={false} onchange={setPersonaModel} testid="persona-model" />
					</div>

					<Tabs
						variant="sub"
						label={m['creator.characters_looks']()}
						value={lookId}
						onchange={(l) => (look = l)}
						tabs={currentPersona.looks.map((l) => ({ id: l.id, label: lookLabel(currentPersona, l.id, textLanguage.names), warning: lookPortraitMoods(l).length === 0, testid: `look-${l.id}` }))}
					>
						{#snippet tab(t)}
							{@const l = currentPersona.looks.find((x) => x.id === t.id)}
							{@const n = l ? lookPortraitMoods(l).length : 0}
							{t.label}{#if l?.default && currentPersona.looks.length > 1}<span class="hint"> · {m['creator.characters_default']()}</span>{/if}
							{#if n}<span class="tally">{m['creator.characters_moods_count']({ count: n })}</span>{:else}<Badge tone="warning">{m['creator.characters_no_images']()}</Badge>{/if}
						{/snippet}
						{#snippet after()}
							<span class="add">
								<input type="text" bind:value={newLookLabel} placeholder={m['creator.characters_new_look']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), newLook())} data-testid="new-look" />
								<button type="button" class="ghost" onclick={newLook} disabled={!newLookLabel.trim()}><Icon name="plus" /> {m['creator.characters_add_look']()}</button>
							</span>
						{/snippet}
					</Tabs>
					<CharacterLook {registry} {moodsDoc} {requestsDoc} {moods} {requests} {character} persona={currentPersona} {lookId} {languages} {source} {bust} onlookchange={(l) => (look = l)} />

					<CharacterTransitions {registry} characterId={id} persona={currentPersona} {languages} />
				</div>
			{/if}
		</section>

		<section class="block" data-testid="apose">
			<h3>{m['creator.characters_apose']()}</h3>
			<AposeEditor character={id} looks={aposeLooks} locale={textLanguage.names} {bust} onchange={setAposePath} />
		</section>

		<section class="block">
			<h3>{m['creator.characters_wanted']()}</h3>
			<WantedImages registry={file} {moods} {requests} locale={textLanguage.names} character={id} onopen={(_, p, l) => ((persona = p), (look = l))} />
		</section>

		<section class="block" data-testid="prompts">
			<h3>{m['creator.prompts_title']()}</h3>
			<CharacterPrompts registry={file} {character} persona={personaId} look={lookId} {moods} {requests} locale={source} />
		</section>
	</article>
{:else}
	<p class="sc-empty pad">{m['creator.characters_pick']()}</p>
{/if}

<style>
	.detail {
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
	}
	.type {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted);
	}
	h2 {
		margin: 0.1rem 0 0.2rem;
	}
	h3 {
		margin: 0 0 0.4rem;
	}
	.relation {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: center;
		margin: 0;
	}
	.block {
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.tally {
		margin-left: 0.35rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.add {
		display: inline-flex;
		gap: 0.25rem;
		align-items: center;
		margin-left: auto;
	}
	.add input {
		width: 12rem;
	}
	.persona {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.model {
		max-width: 24rem;
	}
	.danger {
		color: var(--error);
		align-self: flex-start;
	}
	.pad {
		padding: 1rem;
	}
</style>
