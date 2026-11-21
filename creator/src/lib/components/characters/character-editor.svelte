<script lang="ts">
	import { Section, ToolGroup, type ToolGroupOption } from '@scutoons/ui/editor';
	import {
		REGULAR_PERSONA,
		castsOf,
		defaultLookId,
		findPersona,
		lookLabel,
		personaIds,
		pickText,
		type CharacterRegistry,
		type CharacterView,
		type MoodVocabulary,
		type ToonCast
	} from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { addLook, addPersona, characterLanguages, edit, editCharacter, registryOf, requestsOf, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { lookEdits } from '$lib/editor/look-edit.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { shell, useLentPanels } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CharacterStage from '$lib/components/characters/character-stage.svelte';
	import ExpressionGrid from '$lib/components/characters/expression-grid.svelte';
	import CharacterInspector, { type Level } from '$lib/components/characters/character-inspector.svelte';
	import MoodStrip from '$lib/components/characters/mood-strip.svelte';
	import KanonPanel from '$lib/components/characters/kanon-panel.svelte';
	import { characterSubject, type KanonDocs } from '$lib/editor/kanon-docs.svelte.ts';
	import { shareUses } from '$lib/editor/library-hook.svelte.ts';

	/**
	 * One character as the character workspace (research/redesign/PLAN.md §4, the approved draft workspace.html): the
	 * picture is the background; the Inspector (character › persona › look, Links, Prompts, Wanted), its Kanon tab and the
	 * Moods panel are panel tabs it lends the shell; the Library follows it (its uses).
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
		castDocs,
		kanon,
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
		/** the toons' cast documents (Links edits them; saved with the page) */
		castDocs: Record<string, ContentDoc>;
		/** the page's Kanon texts (saved with the page) */
		kanon: KanonDocs;
		onselect: (id: string) => void;
	} = $props();

	const session = getCreatorSession();
	const file = $derived(registryOf(registry));
	const character = $derived(file?.characters.find((c) => c.id === id) ?? null);
	shareUses(() => (character ? `character:${character.id}` : null), () => character);
	const appearsIn = $derived(character ? castsOf(character.id, casts) : []);
	const langs = $derived(characterLanguages(character, toons, appearsIn));
	const languages = $derived(langs.languages as string[]);
	const source = $derived(langs.source as string);
	const personas = $derived(personaIds(character));
	const personaId = $derived(personas.includes(persona) ? persona : REGULAR_PERSONA);
	const currentPersona = $derived(findPersona(character, personaId));
	const lookIds = $derived((currentPersona?.looks ?? []).map((l) => l.id));
	const lookId = $derived(lookIds.includes(look) ? look : defaultLookId(currentPersona));
	const currentLook = $derived(currentPersona?.looks.find((l) => l.id === lookId));
	const requests = $derived(requestsOf(requestsDoc));
	let mood = $state('neutral');
	let view = $state<CharacterView>('portrait');
	let grid = $state(false);
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);

	const edits = lookEdits(() => ({ registry, moodsDoc, requestsDoc, moods, requests, character: id, persona: currentPersona!, lookId, languages, email: session.user?.email ?? null }));
	const personaLabel = (p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, [source, ...languages]) || p);

	function pickPersona(p: string) {
		if (p !== personaId) {
			persona = p;
			look = '';
		}
	}
	function newPersona(label: string) {
		persona = addPersona(registry, id, label, languages);
		look = '';
	}
	function newLook(label: string) {
		look = addLook(registry, id, personaId, label, languages);
	}
	function personaRemoved() {
		const target = personaId;
		editCharacter(registry, id, (c) => (c.personas = c.personas.filter((p) => p.id !== target)));
		persona = REGULAR_PERSONA;
	}
	async function remove() {
		if (!character) return;
		const used = appearsIn.length ? m['creator.characters_remove_used']({ toons: appearsIn.join(', ') }) : '';
		if (!(await confirmRemove(`${m['creator.characters_remove_confirm']({ name: shownCharacterName(character) })} ${used}`.trim()))) return;
		edit<CharacterRegistry>(registry, (f) => (f.characters = f.characters.filter((c) => c.id !== id)));
		onselect('');
	}

	// ── the Kanon tab: one level's text; a level's square in the Inspector opens it there ──
	let kanonLevel = $state<Level>('character');
	const subjectOf = (l: Level) => characterSubject(id, l === 'character' ? null : personaId, l === 'look' ? lookId : null);
	const kanonSubject = $derived(subjectOf(kanonLevel));
	const kanonDoc = $derived(kanon.docs[kanonSubject] ?? null);
	$effect(() => void kanon.open(kanonSubject, languages, source));
	function openKanon(l: Level) {
		kanonLevel = l;
		shell.open('kanon');
	}
	const LEVELS: ToolGroupOption[] = $derived([
		{ value: 'character', icon: 'user', name: m['creator.characters_character'](), label: m['creator.characters_character']() },
		{ value: 'persona', icon: 'users', name: m['creator.characters_level_persona'](), label: m['creator.characters_level_persona']() },
		{ value: 'look', icon: 'image', name: m['creator.characters_level_look'](), label: m['creator.characters_level_look']() }
	]);

	useLentPanels(() => (character && currentPersona ? { inspector, kanon: kanonTab, moods: currentLook ? moodsTab : null } : {}));
</script>

{#snippet inspector()}
	{#if character && currentPersona}
		<CharacterInspector
			{registry}
			{moodsDoc}
			{requestsDoc}
			{file}
			{moods}
			{requests}
			{character}
			persona={currentPersona}
			{lookId}
			{languages}
			{source}
			{toons}
			casts={castDocs}
			{edits}
			bind:bust
			kanon={(l) => ({ state: kanon.state(subjectOf(l)), open: kanonLevel === l })}
			onpersona={pickPersona}
			onlook={(l) => (look = l)}
			onnewpersona={newPersona}
			onnewlook={newLook}
			onkanon={openKanon}
			onpersonaremoved={personaRemoved}
			onlookremoved={() => (look = '')}
			onremove={remove}
		/>
	{/if}
{/snippet}

{#snippet kanonTab()}
	<div class="kanon" data-testid="character-kanon">
		<ToolGroup label={m['creator.kanon']()} options={LEVELS} value={kanonLevel} expanded onchoose={(v) => (kanonLevel = v as Level)} />
		<Section id="character-kanon-text" label={kanonLevel === 'character' ? (character ? shownCharacterName(character) : '') : kanonLevel === 'persona' ? personaLabel(personaId) : lookLabel(currentPersona, lookId, textLanguage.names)}>
			{#if kanonDoc}<KanonPanel doc={kanonDoc} role={kanonLevel} />{/if}
		</Section>
	</div>
{/snippet}

{#snippet moodsTab()}
	{#if character && currentLook}
		<MoodStrip {character} persona={personaId} look={currentLook} {moods} locale={textLanguage.names} bind:mood bind:view bind:grid {bust} />
	{/if}
{/snippet}

{#if character && currentPersona}
	<article class="char-workspace" data-testid="character-detail" data-id={character.id}>
		{#if currentLook}
			<CharacterStage {character} persona={personaId} look={currentLook} {moods} locale={textLanguage.names} {mood} {view} {grid} {bust} label="{shownCharacterName(character)} · {personaLabel(personaId)} › {lookLabel(currentPersona, lookId, textLanguage.names)}">
				{#snippet gridContent()}
					{#key `${personaId}/${lookId}`}
						<ExpressionGrid {character} persona={personaId} look={currentLook} {moods} {requests} locale={source} onset={edits.setImage} onrequest={edits.request} onaddmood={edits.addMood} {bust} />
					{/key}
				{/snippet}
			</CharacterStage>
		{/if}
	</article>
{:else}
	<p class="sc-empty">{m['creator.characters_pick']()}</p>
{/if}

<style>
	/* the picture stays clear of the Moods strip over the foot */
	.char-workspace {
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: 100%;
		box-sizing: border-box;
		padding-bottom: var(--ws-foot, 0);
		background: var(--editor-stage);
	}
	.kanon {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
	}
	.kanon > :global(:first-child) {
		margin: 0 var(--editor-pad);
		align-self: flex-start;
	}
</style>
