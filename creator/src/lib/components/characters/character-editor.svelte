<script lang="ts">
	import { Divider, Icon, Tabs, panelWidth } from '@scutoons/ui';
	import {
		REGULAR_PERSONA,
		castsOf,
		characterImage,
		defaultLookId,
		findPersona,
		lookLabel,
		lookPortraitMoods,
		personaIds,
		pickText,
		type CharacterRegistry,
		type CharacterView,
		type CreditPerson,
		type MoodVocabulary,
		type RegistryCharacter,
		type ToonCast
	} from '@scutoons/content';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { addLook, addPersona, characterLanguages, edit, editCharacter, isKnownMissing, registryOf, requestsOf, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { lookEdits } from '$lib/editor/look-edit.ts';
	import { SheetDrag } from '$lib/editor/sheet-drag.svelte.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SourceMark from '$lib/components/text/source-mark.svelte';
	import FaceRow from '$lib/components/characters/face-row.svelte';
	import CharacterStage from '$lib/components/characters/character-stage.svelte';
	import CharacterImport from '$lib/components/characters/character-import.svelte';
	import ExpressionGrid from '$lib/components/characters/expression-grid.svelte';
	import CharacterInspector, { type InspectorTab, type Level } from '$lib/components/characters/character-inspector.svelte';
	import KanonSquare from '$lib/components/characters/kanon-square.svelte';
	import { characterSubject, type KanonDocs } from '$lib/editor/kanon-docs.svelte.ts';

	/**
	 * One character (research/CHARACTER-PAGE.md E + K3, owner 2026-10-20): the head (= the character level), the personas
	 * and the looks as face rows, the stage (one picture: persona › look › mood · view; the grid square = every mood to
	 * fill), and the inspector beside it (desktop: a right region with its Divider; phones: a sheet from below whose head
	 * drags it, opened from the bar). The inspector follows the level picked on the page – a tap on the head, a persona,
	 * a look – which the page rings. Each level ends in its Kanon text square (K3, #82): it opens that level's text in the
	 * inspector (no tab of its own). Replaces character-detail.svelte (one long page).
	 * Over 400 lines (RULES "Code"): the page's one layout – head, rows, stage, inspector region and phone sheet – in one
	 * place; its parts (FaceRow, CharacterStage, CharacterInspector, KanonSquare) are components.
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		moods,
		id,
		persona = $bindable(REGULAR_PERSONA),
		look = $bindable(''),
		tab = $bindable<InspectorTab>('profile'),
		grid = $bindable(false),
		sheet = $bindable(false),
		toons,
		casts,
		people,
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
		tab?: InspectorTab;
		grid?: boolean;
		/** phones: the inspector sheet is open (the bar's tab squares open it) */
		sheet?: boolean;
		toons: ToonInfo[];
		casts: Record<string, ToonCast | null>;
		people: CreditPerson[];
		/** the page's Kanon texts (saved with the page) */
		kanon: KanonDocs;
		onselect: (id: string) => void;
	} = $props();

	const session = getCreatorSession();
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
	const currentLook = $derived(currentPersona?.looks.find((l) => l.id === lookId));
	const requests = $derived(requestsOf(requestsDoc));
	let level = $state<Level>('look');
	let mood = $state('neutral');
	let view = $state<CharacterView>('portrait');
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);

	const edits = lookEdits(() => ({ registry, moodsDoc, requestsDoc, moods, requests, character: id, persona: currentPersona!, lookId, languages, email: session.user?.email ?? null }));
	const personaLabel = (p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, [source, ...languages]) || p);
	const face = (ref: { persona?: string; look?: string }) => characterImage(character, ref, 'avatar', { isMissing: isKnownMissing });

	function pickPersona(p: string) {
		if (p !== personaId) {
			persona = p;
			look = '';
		}
		level = 'persona';
	}
	function pickLook(l: string) {
		look = l;
		level = 'look';
	}
	function newPersona(label: string) {
		persona = addPersona(registry, id, label, languages);
		look = '';
		level = 'persona';
	}
	function newLook(label: string) {
		look = addLook(registry, id, personaId, label, languages);
		level = 'look';
	}
	function personaRemoved() {
		const target = personaId;
		editCharacter(registry, id, (c) => (c.personas = c.personas.filter((p) => p.id !== target)));
		persona = REGULAR_PERSONA;
		level = 'persona';
	}
	/** also the phone bar's Delete (characters-editor.svelte, the selection's row) */
	export async function remove() {
		if (!character) return;
		const used = appearsIn.length ? m['creator.characters_remove_used']({ toons: appearsIn.join(', ') }) : '';
		if (!(await confirmRemove(`${m['creator.characters_remove_confirm']({ name: shownCharacterName(character) })} ${used}`.trim()))) return;
		edit<CharacterRegistry>(registry, (f) => (f.characters = f.characters.filter((c) => c.id !== id)));
		onselect('');
	}

	// ── the Kanon (K3): a level's square opens its text in the inspector; the open text follows the picked level ──
	const subjectOf = (l: Level) => characterSubject(id, l === 'character' ? null : personaId, l === 'look' ? lookId : null);
	const kanonSubject = $derived(subjectOf(level));
	function openKanon(l: Level) {
		level = l;
		tab = 'kanon';
		sheet = true;
	}
	$effect(() => {
		if (tab === 'kanon') kanon.open(kanonSubject, languages, source);
	});

	// ── the inspector: desktop = a right region (Divider, remembered width); phones = a sheet from below ──
	let editorEl = $state<HTMLElement>();
	let sheetEl = $state<HTMLElement>();
	const inspectorSize = panelWidth('sc-panel-width:character-inspector', 22, () => editorEl?.getBoundingClientRect().right ?? window.innerWidth);
	inspectorSize.room = () => (editorEl?.clientWidth ?? window.innerWidth) / 2;
	const drag = new SheetDrag(() => sheetEl);
	// the bar opens / closes the sheet (`sheet`); a drag closes it by itself
	$effect(() => {
		if (sheet && !drag.open) drag.open = true;
		if (!sheet && drag.open) drag.close();
	});
	$effect(() => {
		if (!drag.open && sheet) sheet = false;
	});
	const TABS: { id: InspectorTab; icon: string; label: () => string }[] = [
		{ id: 'profile', icon: 'user', label: () => m['creator.characters_tab_profile']() },
		{ id: 'model', icon: 'box', label: () => m['creator.characters_tab_model']() },
		{ id: 'wanted', icon: 'inbox', label: () => m['creator.characters_wanted']() },
		{ id: 'prompts', icon: 'sparkles', label: () => m['creator.prompts_title']() }
	];
	// (the Tabs' `tab` snippet shadows the prop's name in its markup)
	const currentTab = $derived(tab);
	function setTab(id: string) {
		tab = id as InspectorTab;
	}
</script>

{#if character && currentPersona}
	<article class="char-editor" bind:this={editorEl} style:--inspector-width={inspectorSize.css} data-testid="character-detail" data-id={character.id}>
		<div class="main">
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
			<header class="head" class:ringed={level === 'character'} onclick={(e) => !(e.target as HTMLElement).closest('button, a, input') && (level = 'character')}>
				<button type="button" class="avatar" onclick={() => (level = 'character')} aria-label={m['creator.characters_character']()} title={m['creator.characters_character']()} style:background={character.color || null}>
					{#if face({})}<img src={asset(face({}))} alt="" />{/if}
				</button>
				<div class="t">
					<h2 data-testid="character-title">{shownCharacterName(character)}</h2>
					<span class="hint">
						{character.id}{#if appearsIn.length} · {m['creator.characters_appears_in']()}: {#each appearsIn as t, i (t)}{i ? ', ' : ''}<a href="/{t}/cast">{toons.find((x) => x.id === t)?.name ?? t}</a>{/each}{/if}
						· {#each languages as l, i (l)}{i ? ' ' : ''}{l}{#if l === source}<SourceMark />{/if}{/each}
					</span>
				</div>
				<CharacterImport {registry} {moodsDoc} {requestsDoc} {moods} {requests} {character} {languages} bind:bust compact />
				<button type="button" class="sq desktop-only" onclick={remove} title={m['creator.characters_remove']()} aria-label={m['creator.characters_remove']()} data-testid="remove-character"><Icon name="trash-2" /></button>
				<KanonSquare state={kanon.state(subjectOf('character'))} open={tab === 'kanon' && level === 'character'} label={m['creator.kanon']()} onclick={() => openKanon('character')} testid="kanon-character" />
			</header>
			<div class="level-row">
			<FaceRow
				label={m['creator.characters_personas']()}
				items={personas.map((p) => ({ id: p, label: personaLabel(p), src: face({ persona: p }) }))}
				value={personaId}
				ringed={level === 'persona'}
				addLabel={m['creator.characters_persona_label_placeholder']()}
				onpick={pickPersona}
				onadd={newPersona}
				testid="persona"
			/>
			<KanonSquare state={kanon.state(subjectOf('persona'))} open={tab === 'kanon' && level === 'persona'} label={m['creator.kanon']()} onclick={() => openKanon('persona')} testid="kanon-persona" />
			</div>
			{#if currentLook}
				<CharacterStage {character} persona={personaId} look={currentLook} {moods} locale={textLanguage.names} bind:mood bind:view bind:grid {bust} label="{personaLabel(personaId)} › {lookLabel(currentPersona, lookId, textLanguage.names)}">
					{#snippet lead()}
						<FaceRow
							label={m['creator.characters_looks']()}
							items={currentPersona.looks.map((l) => ({ id: l.id, label: lookLabel(currentPersona, l.id, textLanguage.names), src: face({ persona: personaId, look: l.id }), count: String(lookPortraitMoods(l).length), warning: lookPortraitMoods(l).length === 0 }))}
							value={lookId}
							ringed={level === 'look'}
							addLabel={m['creator.characters_new_look']()}
							onpick={pickLook}
							onadd={newLook}
							testid="look"
						/>
					{/snippet}
					{#snippet tools()}
						<KanonSquare state={kanon.state(subjectOf('look'))} open={tab === 'kanon' && level === 'look'} label={m['creator.kanon']()} onclick={() => openKanon('look')} testid="kanon-look" />
					{/snippet}
					{#snippet gridContent()}
						{#key `${personaId}/${lookId}`}
							<ExpressionGrid {character} persona={personaId} look={currentLook} {moods} {requests} locale={source} onset={edits.setImage} onrequest={edits.request} onaddmood={edits.addMood} {bust} />
						{/key}
					{/snippet}
				</CharacterStage>
			{/if}
		</div>
		<Divider size={inspectorSize} edge="left" at="var(--inspector-width, 22rem)" label={m['creator.resize_panel']()} controls="character-inspector" />
		<aside class="side" id="character-inspector" class:open={drag.open} class:dragging={drag.dragging} bind:this={sheetEl} style:--sheet-height={drag.rem !== null ? `${drag.rem}rem` : null} aria-label={m['creator.inspector']()}>
			<div class="insp-tabs">
				<Tabs variant="grid" label={m['creator.inspector']()} value={currentTab} onchange={setTab} tabs={TABS.map((t) => ({ id: t.id, label: t.label(), testid: `inspector-tab-${t.id}` }))}>
					{#snippet tab(t)}
						<Icon name={TABS.find((x) => x.id === t.id)?.icon ?? 'user'} />{#if t.id === 'model'}<span class="b3d" aria-hidden="true">3D</span>{/if}
					{/snippet}
				</Tabs>
			</div>
			<!-- phones: the head drags the sheet (it follows the finger, let go low = closed) -->
			<div class="drag" onpointerdown={drag.down} onpointermove={drag.move} onpointerup={drag.up} onpointercancel={drag.cancel} role="presentation"><span class="grab"></span></div>
			<CharacterInspector
				{tab}
				{level}
				{registry}
				{file}
				{moods}
				{requests}
				{character}
				persona={currentPersona}
				{lookId}
				{languages}
				{source}
				{toons}
				{people}
				{edits}
				{bust}
				personaName={shownCharacterName(character, personaId)}
				lookName={lookLabel(currentPersona, lookId, textLanguage.names)}
				kanonDoc={tab === 'kanon' ? (kanon.docs[kanonSubject] ?? null) : null}
				kanonState={kanon.state(kanonSubject)}
				onpersonaremoved={personaRemoved}
				onlookremoved={() => (look = '')}
			/>
		</aside>
	</article>
{:else}
	<p class="sc-empty">{m['creator.characters_pick']()}</p>
{/if}

<style>
	.char-editor {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) var(--inspector-width, 22rem);
		min-height: 0;
		height: 100%;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		padding: var(--space-075) var(--space-05) var(--space-05) var(--space-1);
		min-width: 0;
		cursor: pointer;
	}
	/* the persona row and its Kanon square: the row scrolls (its fade), the square stays at the end (K3) */
	.level-row {
		display: flex;
		align-items: center;
		min-width: 0;
		padding-right: var(--space-05);
	}
	.level-row > :global(.face-row) {
		flex: 1;
	}
	.avatar {
		all: unset;
		box-sizing: border-box;
		width: 3.5rem;
		height: 3.5rem;
		flex: none;
		overflow: hidden;
		background: var(--surface-hover);
		cursor: pointer;
	}
	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.head.ringed .avatar {
		outline: calc(2 * var(--border-width-regular)) solid var(--ink);
		outline-offset: var(--border-width-regular);
	}
	.t {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}
	h2 {
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.hint {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.sq {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		cursor: pointer;
	}
	.sq:hover,
	.sq:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.side {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		background: var(--paper);
	}
	.insp-tabs {
		flex: none;
		padding: var(--space-05) var(--space-1);
		/* the four icon tabs in one row (the grid tabs' default is 9rem a tab) */
		--tabs-min: 3rem;
	}
	.insp-tabs :global([role='tab']) {
		position: relative;
	}
	/* the model tab's tiny badge (owner 2026-10-20: "3d is fine but add a tiny 3D badge") */
	.b3d {
		position: absolute;
		right: calc(50% - 1.5rem);
		top: 0.15rem;
		padding: 0 0.1rem;
		font-size: var(--type-nav-micro-size);
		line-height: 1.1;
		background: var(--ink);
		color: var(--paper);
	}
	.insp-tabs :global([aria-selected='true'] .b3d) {
		background: var(--paper);
		color: var(--ink);
	}
	.drag {
		display: none;
	}
	@media (max-width: 767px) {
		.desktop-only {
			display: none;
		}
		.char-editor {
			grid-template-columns: minmax(0, 1fr);
		}
		.main {
			border-right: 0;
		}
		/* the inspector = a sheet from below, over the stage; closed it is gone (the bar's squares open it) */
		.side {
			position: absolute;
			z-index: 4;
			left: 0;
			right: 0;
			bottom: 0;
			/* as tall as its content (at most 60 %) until dragged: then exactly the dragged height */
			height: var(--sheet-height, auto);
			max-height: var(--sheet-height, 60%);
			border-top: var(--border-width-regular) solid var(--ink);
			transform: translateY(100%);
			visibility: hidden;
			transition: transform 0.2s ease, visibility 0.2s;
		}
		.side.open {
			transform: none;
			visibility: visible;
		}
		.side.dragging {
			transition: none;
		}
		/* the bar holds the tabs on a phone */
		.insp-tabs {
			display: none;
		}
		.drag {
			display: flex;
			justify-content: center;
			align-items: center;
			height: 1.75rem;
			touch-action: none;
			flex: none;
		}
		.grab {
			width: 2.5rem;
			height: 0.25rem;
			background: var(--border-muted);
		}
		.head {
			padding: var(--space-05) var(--space-1);
		}
		.avatar {
			width: 2.75rem;
			height: 2.75rem;
		}
	}
</style>
