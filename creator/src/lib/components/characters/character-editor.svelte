<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Section, ToolGroup, type ToolGroupOption } from '@scutoons/ui/editor';
	import {
		REGULAR_PERSONA,
		castsOf,
		defaultLookId,
		findPersona,
		lookLabel,
		personaIds,
		personaModelSource,
		pickText,
		type CharacterRegistry,
		type CharacterView,
		type MoodVocabulary,
		type ToonCast
	} from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { addLook, addPersona, characterLanguages, edit, editCharacter, registryOf, requestsOf, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editHref } from '$lib/editor/edit-key.ts';
	import { lookEdits } from '$lib/editor/look-edit.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { CharacterModel } from '$lib/editor/character-model.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { shell, useTabs } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CharacterStage from '$lib/components/characters/character-stage.svelte';
	import CharacterViews, { type CharacterViewId } from '$lib/components/characters/character-views.svelte';
	import CharacterModelCanvas from '$lib/components/characters/character-model-canvas.svelte';
	import CharacterRigPanel from '$lib/components/characters/character-rig-panel.svelte';
	import ExpressionGrid from '$lib/components/characters/expression-grid.svelte';
	import CharacterInspector, { type Level } from '$lib/components/characters/character-inspector.svelte';
	import CharacterLinks from '$lib/components/characters/character-links.svelte';
	import CharacterPrompts from '$lib/components/characters/character-prompts.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import MoodStrip from '$lib/components/characters/mood-strip.svelte';
	import KanonPanel from '$lib/components/characters/kanon-panel.svelte';
	import { characterSubject, type KanonDocs } from '$lib/editor/kanon-docs.svelte.ts';
	import { shareUses } from '$lib/editor/library-hook.svelte.ts';

	/**
	 * One character as the character workspace (research/redesign/characters.html, A with B's 3D place and C's split):
	 * its sections are the tabs of one panel – Kanon (who it is, its texts), Expressions, Rig, Voice, Cast, Generations –
	 * and its own view fills the stage: Sheet, Expressions or 3D, S splits it. The Library follows it (its uses).
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
		vocabulary,
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
		/** the toons' cast documents (Cast edits them; saved with the page) */
		castDocs: Record<string, ContentDoc>;
		/** the page's Kanon texts (saved with the page) */
		kanon: KanonDocs;
		/** the mood words (every character's), under the Expressions tab */
		vocabulary: Snippet;
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
	let pictureView = $state<CharacterView>('portrait');
	let view = $state<CharacterViewId>('sheet');
	let split = $state(false);
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);

	const edits = lookEdits(() => ({ registry, moodsDoc, requestsDoc, moods, requests, character: id, persona: currentPersona!, lookId, languages, email: session.user?.email ?? null }));
	const personaLabel = (p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, [source, ...languages]) || p);
	const where = $derived(currentPersona ? `${personaLabel(personaId)} › ${lookLabel(currentPersona, lookId, textLanguage.names)}` : '');

	// ── the look's model: the 3D place and the Rig tab read it once ──
	const model = new CharacterModel();
	const modelSource = $derived(character ? personaModelSource(character, personaId, lookId) : { path: '', fallback: true });
	const attached = $derived<'look' | 'persona' | null>(modelSource.fallback ? null : currentLook?.model?.trim() ? 'look' : 'persona');
	$effect(() => void model.read(modelSource.fallback ? '' : modelSource.path));
	function playClip(clip: number) {
		model.play(clip);
		if (view !== '3d' && !split) view = '3d';
	}

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

	// ── the Kanon tab: who it is, then one level's text; a level's square opens it there ──
	let kanonLevel = $state<Level>('character');
	const subjectOf = (l: Level) => characterSubject(id, l === 'character' ? null : personaId, l === 'look' ? lookId : null);
	const kanonSubject = $derived(subjectOf(kanonLevel));
	const kanonDoc = $derived(kanon.docs[kanonSubject] ?? null);
	$effect(() => void kanon.open(kanonSubject, languages, source));
	function openKanon(l: Level) {
		kanonLevel = l;
		shell.open('kanon');
		document.getElementById('character-kanon-text')?.scrollIntoView({ block: 'nearest' });
	}
	const LEVELS: ToolGroupOption[] = $derived([
		{ value: 'character', icon: 'user', name: m['creator.characters_character'](), label: m['creator.characters_character']() },
		{ value: 'persona', icon: 'users', name: m['creator.characters_level_persona'](), label: m['creator.characters_level_persona']() },
		{ value: 'look', icon: 'image', name: m['creator.characters_level_look'](), label: m['creator.characters_level_look']() }
	]);

	useTabs(() => (character && currentPersona ? { kanon: kanonTab, expressions: currentLook ? expressionsTab : null, rig: currentLook ? rigTab : null, voice: voiceTab, cast: castTab, generations: generationsTab } : {}));
</script>

{#snippet kanonTab()}
	{#if character && currentPersona}
		<CharacterInspector
			{registry}
			{moodsDoc}
			{requestsDoc}
			{moods}
			{requests}
			{character}
			persona={currentPersona}
			{lookId}
			{languages}
			{source}
			{toons}
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
		<div class="kanon" data-testid="character-kanon">
			<ToolGroup label={m['creator.kanon']()} options={LEVELS} value={kanonLevel} expanded onchoose={(v) => (kanonLevel = v as Level)} />
			<Section id="character-kanon-text" label={kanonLevel === 'character' ? shownCharacterName(character) : kanonLevel === 'persona' ? personaLabel(personaId) : lookLabel(currentPersona, lookId, textLanguage.names)}>
				{#if kanonDoc}<KanonPanel doc={kanonDoc} role={kanonLevel} />{/if}
			</Section>
		</div>
	{/if}
{/snippet}

{#snippet expressionsTab()}
	{#if character && currentLook}
		<MoodStrip {character} persona={personaId} look={currentLook} {moods} locale={textLanguage.names} bind:mood bind:view={pictureView} bind:grid={() => view === 'expressions', (on) => (view = on ? 'expressions' : 'sheet')} {bust} />
		<Section id="character-mood-words" label={m['shell.panel_vocabulary']()} open={false}>
			{@render vocabulary()}
		</Section>
	{/if}
{/snippet}

{#snippet rigTab()}
	{#if character && currentPersona && currentLook}
		<CharacterRigPanel {registry} {character} persona={currentPersona} look={currentLook} {edits} {bust} {model} onplay={playClip} />
	{/if}
{/snippet}

{#snippet voiceTab()}
	{#if character}
		<div class="say" data-testid="character-voice">
			<p class="sc-empty">{m['character.voice_none']()}</p>
			<a href={editHref(`rig:character.${character.id}`, { w: 'sound' })} data-testid="character-sound-design">{m['creator.characters_sound_design']()}</a>
		</div>
	{/if}
{/snippet}

{#snippet castTab()}
	{#if character}
		<div class="say"><CharacterLinks {character} {registry} casts={castDocs} {toons} /></div>
	{/if}
{/snippet}

{#snippet generationsTab()}
	{#if character && currentPersona}
		<Section id="character-prompts" label={m['creator.prompts_title']()}>
			<CharacterPrompts registry={file} {character} persona={personaId} look={lookId} {moods} {requests} locale={source} />
		</Section>
		<Section id="character-wanted" label={m['creator.characters_wanted']()}>
			<WantedImages registry={file} {moods} {requests} locale={source} character={character.id} persona={personaId} look={lookId} />
		</Section>
	{/if}
{/snippet}

{#if character && currentPersona}
	<article class="char-workspace" data-testid="character-detail" data-id={character.id}>
		{#if currentLook}
			<CharacterViews bind:view bind:split>
				{#snippet sheet()}
					<CharacterStage {character} persona={personaId} look={currentLook} {moods} locale={textLanguage.names} {mood} view={pictureView} {bust} label="{shownCharacterName(character)} · {where}" />
				{/snippet}
				{#snippet expressions()}
					<div class="grid-area" data-testid="character-expressions">
						{#key `${personaId}/${lookId}`}
							<ExpressionGrid {character} persona={personaId} look={currentLook} {moods} {requests} locale={source} onset={edits.setImage} onrequest={edits.request} onaddmood={edits.addMood} {bust} />
						{/key}
					</div>
				{/snippet}
				{#snippet place(room: { left: number; right: number })}
					<CharacterModelCanvas {model} {where} {attached} {room} onattach={(path) => edits.setModel(path)} />
				{/snippet}
			</CharacterViews>
		{/if}
	</article>
{:else}
	<p class="sc-empty">{m['creator.characters_pick']()}</p>
{/if}

<style>
	.char-workspace {
		flex: 1;
		min-height: 0;
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
	.say {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.say .sc-empty {
		margin: 0;
	}
	/* the expressions on the stage: under the bar, clear of the panels and the view pill */
	.grid-area {
		position: absolute;
		inset: 0;
		overflow: auto;
		box-sizing: border-box;
		background: var(--editor-surface);
		padding: calc(var(--shell-top) + 1rem) var(--in-r) calc(var(--status-room) + var(--control-height) + 1.5rem) var(--in-l);
	}
</style>
