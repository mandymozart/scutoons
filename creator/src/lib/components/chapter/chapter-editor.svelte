<script lang="ts">
	import type { VariantMedia } from '@scutoons/media';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { onMount, tick, untrack } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { ActionBar, BarTabs, PhoneBar, type PhoneBarItem, Divider, Icon, MaximizeButton, panelWidth, shellDock, useDock } from '@scutoons/ui';
	import { liveRegion, region } from '@scutoons/ui';
	import { beforeNavigate, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { dockWidth, flowWidth, usePageNav } from '$lib/nav.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import { editorKeys, isField } from '$lib/editor/editor-keys.ts';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { defaultMoodVocabulary, entities, getAt, deleteAt, scriptLabels, type Path, type ToonLanguages } from '@scutoons/content';
	import { castEntries, castIndex, chapterSceneCharacters, characterIndex, expressionIndex, featuringSuggestions, REGULAR_PERSONA, toonStar, validateCast } from '@scutoons/content';
	import {
		addToSeasonCast,
		castDoc,
		castOf,
		castWithStar,
		loadToons,
		moodsDoc,
		moodsOf,
		registryDoc,
		registryOf,
		type ToonInfo
	} from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonStatusOf, syncChapter } from '$lib/editor/visibility.ts';
	import { validateFile, validateNeutral, chapterReach, scriptJumps, toonFacts, factNames, type ChapterReach, type FlowNode, type Problem, type ToonFacts } from '@scutoons/content';
	import { loadIncoming, loadToonDocs } from '$lib/editor/toon-facts.ts';
	import { JumpPlaces, setJumpPlaces } from '$lib/editor/jump-places.svelte.ts';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import { type Tool } from '$lib/components/slice/slice-canvas.svelte';
	import StripCanvas from '$lib/components/slice/strip-canvas.svelte';
	import PhoneKinds from '$lib/components/slice/phone-kinds.svelte';
	import LongPressMenu from '$lib/components/slice/long-press-menu.svelte';
	import { TEXT_PRESETS } from '$lib/editor/text-presets.ts';
	import { Menu } from '@scutoons/ui';
	import { piecePlace } from '$lib/editor/canvas-pieces.ts';
	import type { PickedPiece } from '$lib/editor/collage-pick.ts';
	import { followPieces, piecesMoved } from '$lib/editor/collage-edit.ts';
	import { pieceLabel } from '$lib/editor/collage-words.ts';
	import { legacyTextCount, legacyPlan, applyLegacyPlan, type TextPreset } from '$lib/editor/text-presets.ts';
	import { itemCatalog } from '$lib/editor/catalog.svelte.ts';
	import { ChapterView, clampZoom, type InspectorTab, type Section } from '$lib/editor/chapter-state.svelte.ts';
	import * as edits from '$lib/editor/slice-edit.ts';
	import { clipboard, clipboardEvents, useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { slicesClipHandler } from '$lib/editor/slices-clip.svelte.ts';
	import { groupsClipHandler } from '$lib/editor/groups-clip.svelte.ts';
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { sliceClipHandler } from '$lib/editor/slice-clip.ts';
	import { setClipContext } from '$lib/editor/clip-context.ts';
	import { countPlacements } from '$lib/editor/clip.ts';
	import { itemName } from '$lib/editor/catalog.svelte.ts';
	import { api } from '$lib/api.ts';
	import type { ClipResult } from '$lib/editor/clipboard.svelte.ts';
	import ChapterTree from '$lib/components/chapter/chapter-tree.svelte';
	import SliceToolbar from '$lib/components/slice/slice-toolbar.svelte';
	import SliceInspector from '$lib/components/slice/slice-inspector.svelte';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import CollagePickerSheet from '$lib/components/collage/collage-picker-sheet.svelte';
	import { mapDoc as worldMapDoc } from '$lib/editor/map.ts';
	import { judgedPresets, ownPresets, presetsDoc } from '$lib/editor/presets-doc.ts';
	import MetaPanel from '$lib/components/chapter/meta-panel.svelte';
	import FeaturingField from '$lib/components/fields/featuring-field.svelte';
	import RelatedItemsField from '$lib/components/fields/related-items-field.svelte';
	import GroupPanel from '$lib/components/chapter/group-panel.svelte';
	import AudioPanel from '$lib/components/chapter/audio-panel.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsDock from '$lib/components/page/problems-dock.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import FlowDock from '$lib/components/flow/flow-dock.svelte';
	import ReaderDock from '$lib/components/reader/reader-dock.svelte';
	import { toonUrl } from '$lib/utils/cdn.ts';
	import ClipMenu from '$lib/components/page/clip-menu.svelte';
	import { toast } from '$lib/toasts.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { exprContext, fieldKey } from '$lib/editor/expr-context.svelte.ts';

	/** `languages`: the toon's source + languages (ToonIndex, loaded by the page) – every file is edited in these */
	let { toon, chapterNumber, languages }: { toon: string; chapterNumber: number; languages: ToonLanguages } = $props();

	const panelSize = dockWidth('chapter', 16); // normal = --tool-panel-width
	// the Flow panel's graph needs more room from the start (owner 2026-10-06: drags wider up to half the window too)
	const flowSize = flowWidth();
	// the Reader (BACKLOG 10.49): a phone screen reads well from 24rem
	const readerSize = dockWidth('reader', 24);
	// the slice inspector: a right region of the editor's main area (agents/DOCKING.md) – its Divider drags it from its
	// normal 22rem up to half the area (owner 2026-10-03), remembered; under 1280px it sits below the canvas
	let columnsEl = $state<HTMLElement>();
	const inspectorSize = panelWidth('sc-panel-width:slice-inspector', 22, () => columnsEl?.getBoundingClientRect().right ?? window.innerWidth);
	inspectorSize.room = () => (columnsEl?.clientWidth ?? window.innerWidth) / 2;
	// maximized (owner 2026-10-06, agents/DOCKING.md "Maximize"): the inspector fills the editor's columns and the shell
	// gives the content the dock's room too; the canvas is only hidden (its edits, scroll and size stay)
	const INSPECTOR = 'slice-inspector';
	/**
	 * phones (BACKLOG 10.50 e): the inspector is a sheet below the canvas – its grab handle drags it taller / lower (rem,
	 * not remembered), a tap switches between the normal height (45 %) and tall (80 %); arrows step it
	 */
	let sheetRem = $state<number | null>(null);
	let sheetDrag: { id: number; y: number; moved: boolean } | null = null;
	const rootRem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	function sheetLimits() {
		const h = (columnsEl?.clientHeight ?? window.innerHeight) / rootRem();
		return { min: 3.5, max: Math.max(3.5, h - 6), h };
	}
	function setSheet(rem: number) {
		const { min, max } = sheetLimits();
		sheetRem = Math.round(Math.min(max, Math.max(min, rem)) * 4) / 4;
	}
	function sheetDown(event: PointerEvent) {
		sheetDrag = { id: event.pointerId, y: event.clientY, moved: false };
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}
	function sheetMove(event: PointerEvent) {
		if (!sheetDrag || sheetDrag.id !== event.pointerId || !columnsEl) return;
		if (!sheetDrag.moved && Math.abs(event.clientY - sheetDrag.y) < 6) return;
		sheetDrag.moved = true;
		setSheet((columnsEl.getBoundingClientRect().bottom - event.clientY) / rootRem());
	}
	function sheetUp(event: PointerEvent) {
		if (!sheetDrag || sheetDrag.id !== event.pointerId) return;
		const tap = !sheetDrag.moved;
		sheetDrag = null;
		if (!tap) return;
		const { h } = sheetLimits();
		setSheet(sheetRem !== null && sheetRem > h * 0.6 ? h * 0.45 : h * 0.8);
	}
	function sheetKey(event: KeyboardEvent) {
		const step = event.key === 'ArrowUp' ? 2 : event.key === 'ArrowDown' ? -2 : 0;
		if (!step) return;
		event.preventDefault();
		setSheet((sheetRem ?? sheetLimits().h * 0.45) + step);
	}
	const inspectorMax = $derived(shellDock.maximized === INSPECTOR);
	$effect(() => () => {
		if (shellDock.maximized === INSPECTOR) shellDock.restore();
	});
	// leaving the slice (the chapter's meta, a group) leaves no inspector to fill the page
	$effect(() => {
		if (inspectorMax && view.section !== 'slice') shellDock.restore();
	});

	// ── Documents ───────────────────────────────────────────────────────────────────────────────────────
	// the page re-creates this component per chapter and toon languages ({#key}), so the initial props are the right ones
	// svelte-ignore state_referenced_locally
	const chapter = toonDoc(entities.chapter(toon, chapterNumber), languages);
	// the season holds the chapter's status (visibility, per locale): edited in the meta panel, saved with the chapter
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, 1), languages);
	// the season's cast + the character registry: the VN character fields pick from every character (the season's cast
	// first); picking someone who isn't cast adds them to the season's cast (saved / published with the chapter)
	/** the chapter's season (the editor edits season 1) */
	const SEASON = 1;
	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	const moods = moodsDoc();
	// the custom collage presets (research/COLLAGE.md §12.4): the canvas draws, the fields edit, the checks judge the
	// pieces with its newest draft; saved / published with the chapter while edited here
	const colorations = presetsDoc();
	const collagePresets = $derived(ownPresets(colorations));
	// the world map: read for the expressions' map facts (places are unlocked by unlockLocation events – owner 2026-10-11)
	const worldMap = worldMapDoc();
	/** the world map's place ids (unlockLocation events point at them); undefined until it is loaded = no check */
	const worldMapPlaces = $derived.by(() => {
		const map = worldMap.loading ? null : (worldMap.data?.[worldMap.primary] as { locations?: { id: string; toons?: string[] }[] } | undefined);
		return map?.locations ? map.locations : undefined;
	});
	/** the world map's region ids (migrate events point at them); undefined until loaded = no check */
	const worldMapRegions = $derived.by(() => {
		const map = worldMap.loading ? null : (worldMap.data?.[worldMap.primary] as { regions?: { id: string }[] } | undefined);
		return map?.regions ? map.regions.map((r) => r.id) : undefined;
	});
	let toonInfos = $state<ToonInfo[]>([]);
	let castTouched = $state(false);
	const registryIndex = $derived(characterIndex(registryOf(registry)));
	const hasCast = $derived(!!castOf(cast) && !!registryOf(registry));
	const seasonCast = $derived(castEntries(castOf(cast), SEASON));
	const characters = $derived<{ name: string; label?: string }[]>(
		hasCast ? seasonCast.map((e) => ({ name: e.character, label: shownCharacterName(registryIndex.get(e.character)) || e.character })) : []
	);
	function ensureCast(character: string, persona: string) {
		const changed = addToSeasonCast(cast, SEASON, character, persona);
		if (!changed) return;
		castTouched = true;
		// a toast with Undo (E.1): the cast change is undone, the chapter edit stays
		toast(m['creator.characters_cast_added']({ name: shownCharacterName(registryIndex.get(character), persona) || character, season: m['creator.season']({ number: SEASON }) }), {
			action: { label: m['creator.undo'](), run: () => cast.undo() }
		});
	}
	const choices = $derived<CharacterChoices | undefined>(
		registry.loading || cast.loading
			? undefined
			: {
					registry: registryOf(registry),
					moods: moodsOf(moods),
					toons: toonInfos,
					cast: seasonCast,
					season: m['creator.season']({ number: SEASON }),
					// names, looks and moods in the header's text language (BACKLOG 10.51)
					locale: textLanguage.names,
					ensureCast
				}
	);
	let images = $state<string[]>([]);

	const chapterData = $derived(chapter.data[chapter.primary]);
	const groups = $derived<any[]>(chapterData?.groups ?? []);
	/** the toon's star (always in the cast, first in the suggested featuring) */
	const star = $derived(toonStar(registryOf(registry), toon));
	/**
	 * A slice sample picked as files (asset picker / upload; `src` = the 128k default, `variants` = its levels): its
	 * sample-library id. An entry with that file gets the levels; a file not in the chapter's library yet is added to it
	 * (id from the file name: s1c1-choir-loud.128.mp3 → choir_loud) – saved with the chapter.
	 */
	function librarySample(media: VariantMedia): string {
		const path = media.src ?? '';
		const library: any[] = chapterData?.audio?.sampleLibrary ?? [];
		const known = library.find((s) => s?.path === path || (media.variants && Object.values(s?.variants ?? {}).includes(path)));
		if (known?.id) {
			chapter.edit((file) => {
				const entry = (file.audio?.sampleLibrary ?? []).find((s: any) => s?.id === known.id);
				if (!entry) return;
				entry.path = path;
				if (media.variants) entry.variants = media.variants;
				else delete entry.variants;
			});
			return known.id;
		}
		const base = (path.split('/').pop() ?? path).replace(/(\.\d+)?\.[a-z0-9]+$/i, '').replace(/^s\d+c\d+-/i, '').replace(/[^a-z0-9]+/gi, '_').replace(/^_+|_+$/g, '').toLowerCase() || 'sample';
		let id = base;
		for (let n = 2; library.some((s) => s?.id === id); n++) id = `${base}_${n}`;
		chapter.edit((file) => {
			file.audio ??= {};
			file.audio.sampleLibrary ??= [];
			if (!file.audio.sampleLibrary.some((s: any) => s?.id === id)) file.audio.sampleLibrary.push(media.variants ? { id, path, variants: media.variants } : { id, path });
		});
		return id;
	}
	/**
	 * The music level each track holds when the active slice comes into view: the last value an earlier slice set (the
	 * reader keeps a track's level until a slice changes it; every track starts silent – AdaptiveMusicPlayer).
	 */
	const heldMusic = $derived.by(() => {
		const out: Record<string, { volume: number; label: string }> = {};
		if (view.section !== 'slice' || view.sliceIndex < 0) return out;
		const all = groups;
		for (let gi = 0; gi <= view.groupIndex && gi < all.length; gi++) {
			const slices: any[] = all[gi]?.slices ?? [];
			const until = gi === view.groupIndex ? view.sliceIndex : slices.length;
			for (let si = 0; si < until; si++) {
				for (const t of slices[si]?.audio?.music?.tracks ?? []) {
					if (t?.id && typeof t.volume === 'number') out[t.id] = { volume: t.volume, label: gi === view.groupIndex ? `slice ${si + 1}` : `${groupName(groups[gi], gi)} · ${si + 1}` };
				}
			}
		}
		return out;
	});
	const allDocs = $derived<ContentDoc[]>([
		chapter,
		season,
		...(castTouched || cast.dirty ? [cast] : []),
		...(worldMap.dirty ? [worldMap] : []),
		...(colorations.dirty ? [colorations] : [])
	]);
	const anyDirty = $derived(allDocs.some((d) => d.dirty));
	/**
	 * the Recover bars' documents: the page's, and the presets while clean – autosaved only while dirty, so a reload or a
	 * crash leaves a copy that the load offers next to the chapter's, whose pieces may name its ids (research/COLLAGE.md
	 * §12.4); not in allDocs: a never-stored presets document is not "missing"
	 */
	const noticeDocs = $derived(colorations.dirty ? allDocs : [...allDocs, colorations]);
	/** the chapter's documents with slices (one document per chapter since 2026-10-19) */
	const sliceDocs = $derived<ContentDoc[]>([chapter]);
	/** texts that still have the old look (a mask over baked text, chapter 1) */
	const legacyTexts = $derived(sliceDocs.reduce((n, d) => n + (d.loading ? 0 : legacyTextCount(d.data[d.primary])), 0));
	/** one step (owner 2026-10-03): every masked text → its preset; save / publish as usual */
	function convertLegacyTexts() {
		for (const d of sliceDocs) {
			const plan = legacyPlan($state.snapshot(d.data[d.primary]));
			if (plan.length) d.edit((file) => applyLegacyPlan(file, plan), 'convert-presets');
		}
	}
	const anySaving = $derived(allDocs.some((d) => d.saving));

	onMount(async () => {
		await Promise.all([chapter.load(), season.load(), cast.load(), registry.load(), moods.load(), worldMap.load(), colorations.load()]);
		loadToons(getLocale()).then((list) => (toonInfos = list));
		refreshImages();
		// the rest of the toon (reachability: labels something jumps to, variables something sets) – without it no
		// condition is judged and every labelled group counts as reachable
		// + the labels other toons jump to (the server's flow index)
		Promise.all([loadToonDocs(toon, new Set(sliceDocs.map((d) => d.entity))), loadIncoming(toon)])
			.then(([docs, incoming]) => (toonDocsFacts = toonFacts(docs, { toon, incoming: incoming.labels, elsewhere: incoming.elsewhere })))
			.catch(() => (toonDocsFacts = null));
	});

	// ── Reachability (owner 2026-10-06, @scutoons/content reach.ts) ─────────────────────────────────────────────────
	let toonDocsFacts = $state<ToonFacts | null>(null);
	/** the toon's facts: the other documents (loaded once) + this chapter's own, live */
	const facts = $derived.by<ToonFacts | undefined>(() => {
		const rest = toonDocsFacts;
		if (!rest) return undefined;
		const live = toonFacts([...sliceDocs.map((d) => $state.snapshot(d.data[d.primary])), $state.snapshot(worldMap.data?.[worldMap.primary])], { toon });
		return { toon, variables: new Set([...rest.variables, ...live.variables]), labels: new Set([...rest.labels, ...live.labels]), ...(rest.reads ? { reads: rest.reads } : {}) };
	});
	// the expression and variable fields: the story variables of every toon as written here (`x`, `global.x`,
	// `toptoy.x` – one save game; the builder and the variable typeahead offer them, unknown names warn) and the toons
	$effect(() => exprContext.setVariables(facts ? factNames(facts) : null));
	$effect(() => {
		exprContext.toon = toon;
		exprContext.toons = toonInfos.map((t) => ({ id: t.id, name: t.name }));
	});
	$effect(() => () => exprContext.setVariables(null));
	/** the chapter's reachability for the labels in the tree / slice list / event lists – debounced while editing */
	let reach = $state<ChapterReach | null>(null);
	/** the groups `reach` was computed from (the Flow panel draws exactly what the chips say) */
	let reachGroups = $state.raw<any[] | null>(null);
	$effect(() => {
		const snapshot = $state.snapshot(groups);
		const f = facts;
		const timer = setTimeout(() => {
			reach = chapterReach(snapshot, f);
			reachGroups = snapshot;
		}, 250);
		return () => clearTimeout(timer);
	});
	/** chapter path (groups.g.…) → its mark */
	const marks = $derived.by(() => {
		const out = new Map<string, ReachMark>();
		const r = reach;
		if (!r) return out;
		r.groups.forEach((x, gi) => x === 'unreachable' && out.set(`groups.${gi}`, 'unreachable'));
		r.slices.forEach((list, gi) => list.forEach((x, si) => x === 'unreachable' && r.groups[gi] !== 'unreachable' && out.set(`groups.${gi}.slices.${si}`, 'unreachable')));
		r.events.forEach((list, gi) => list.forEach((x, i) => x === 'unreachable' && r.groups[gi] !== 'unreachable' && out.set(`groups.${gi}.events.${i}`, 'unreachable')));
		for (const p of r.loops) out.set(p.join('.'), 'loop');
		for (const p of r.neverTrue) out.set(p.slice(0, -1).join('.'), 'never');
		for (const p of r.stuck ?? []) out.set(p.join('.'), 'stuck');
		return out;
	});
	const markAt = (...path: (string | number)[]): ReachMark => marks.get(path.join('.')) ?? null;

	function refreshImages() {
		knownImages(toon, chapter.textLanguages, allDocs.map((d) => $state.snapshot(d.data))).then((list) => (images = list));
	}

	// ── Selection / view state: one object (lib/editor/chapter-state.svelte.ts) – the tree, the canvas, the inspector
	// and the keyboard write it
	const view = new ChapterView();
	// the canvas and the inspector show the top bar's TEXT language (default the source; owner
	// 2026-10-06: one language everywhere, the header's switch the only one)
	const displayLocale = $derived(textLanguage.current(languages.languages, languages.source));
	// snap, grid size, zoom and the seams toggle are remembered on this device
	$effect(() => {
		void view.snap;
		void view.gridSize;
		void view.zoom;
		void view.seams;
		view.savePrefs();
	});
	// the Flow / Problems / Reader the last page docked stay open here (taken over, not remounted – UI-CONCEPTS E.3 b);
	// `?flow=1` (a deep link) opens the Flow. Several at once (10.57): each its own panel, side by side
	type PanelKind = 'problems' | 'history' | 'flow' | 'reader';
	const open = $state<Record<PanelKind, boolean>>({
		flow: page.url.searchParams.get('flow') === '1' || shellDock.has('flow'),
		problems: shellDock.has('problems'),
		reader: shellDock.has('reader'),
		history: false
	});
	// docked in the shell's right dock (agents/DOCKING.md): the page makes room; with Ask open too the sidebar folds. The
	// Flow, Problems and the Reader are mounted by the shell (`kind` + `content`): the next page of the same kind takes
	// them over
	const PANELS = { problems: { icon: 'alert-triangle', title: () => m['creator.problems']() }, history: { icon: 'clock', title: () => m['creator.history']() }, flow: { icon: 'git-merge', title: () => m['creator.flow']() }, reader: { icon: 'smartphone', title: () => m['creator.reader']() } };
	const flowProps = () => ({
		input: {
			level: 'chapter' as const,
			groups: reachGroups,
			reach,
			chapter: chapterNumber,
			current: view.section === 'group' || view.section === 'slice' ? view.groupIndex : -1,
			slice: view.section === 'slice' ? view.sliceIndex : -1,
			who: flowWho,
			toon,
			registry: registryOf(registry),
			facts,
			nextChapter: flowNextChapter,
			name: flowName,
			text: flowText,
			onopen: openFlowNode
		},
		onclose: () => (open.flow = false)
	});
	// the Reader (UI-CONCEPTS E.6): the viewer's draft preview at the open group / slice (the chapter's start on its meta /
	// audio), in the text language when the reader has it (de / en), else the first it has
	const readerLocale = $derived(displayLocale === 'de' || displayLocale === 'en' ? displayLocale : (chapter.textLanguages.find((l) => l === 'de' || l === 'en') ?? 'de'));
	const readerProps = () => ({
		place: {
			toon,
			chapter: chapterNumber,
			group: view.section === 'group' || view.section === 'slice' ? Math.max(0, view.groupIndex) : 0,
			slice: view.section === 'slice' ? view.sliceIndex : -1
		},
		locale: readerLocale,
		onclose: () => (open.reader = false)
	});
	const problemsProps = () => ({ problems, docs: allDocs, home: chapter.entity, groupLabel: (i: number) => groupName(groups[i], i), ongoto: gotoProblem, onclose: () => (open.problems = false) });
	const docked = (kind: PanelKind, size: typeof panelSize) => ({ kind, size, title: PANELS[kind].title(), icon: PANELS[kind].icon, close: () => (open[kind] = false) });
	useDock(() => (open.flow ? { ...docked('flow', flowSize), content: { component: FlowDock, props: flowProps } } : null));
	useDock(() => (open.reader ? { ...docked('reader', readerSize), content: { component: ReaderDock, props: readerProps } } : null));
	useDock(() => (open.problems ? { ...docked('problems', panelSize), content: { component: ProblemsDock, props: problemsProps } } : null));
	useDock(() => (open.history ? { ...docked('history', panelSize), panel: sidePanel } : null));
	function togglePanel(next: PanelKind) {
		open[next] = !open[next];
	}
	let canvas = $state<StripCanvas | null>(null);
	/**
	 * Ctrl / Cmd + wheel over the slice area zooms the canvas (also with the zoom flyout closed; a trackpad's pinch is the
	 * same event), 50–300 %, the point under the pointer stays put (UI-CONCEPTS E.4). Non-passive: it replaces the
	 * browser's page zoom there.
	 */
	function wheelZoom(area: HTMLElement) {
		const onWheel = async (event: WheelEvent) => {
			if (!event.ctrlKey && !event.metaKey) return;
			event.preventDefault();
			const el = area.querySelector<HTMLElement>('[data-testid="canvas-wrap"]');
			if (!el) return;
			const before = el.getBoundingClientRect();
			const fx = (event.clientX - before.left) / (before.width || 1);
			const fy = (event.clientY - before.top) / (before.height || 1);
			const step = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
			const next = clampZoom(view.shownZoom * Math.exp(-step * 0.002));
			if (next === view.shownZoom && view.zoom !== null) return;
			view.zoom = next;
			view.shownZoom = next;
			await tick();
			const after = el.getBoundingClientRect();
			area.scrollLeft += after.left + fx * after.width - event.clientX;
			area.scrollTop += after.top + fy * after.height - event.clientY;
		};
		area.addEventListener('wheel', onWheel, { passive: false });
		return () => area.removeEventListener('wheel', onWheel);
	}
	/** save / publish / conflict / the notice (lib/editor/doc-session.svelte.ts – the same as the meta pages') */
	const session = new DocSession({
		docs: () => allDocs,
		// Publish lists the chapter and its files with drafts (BACKLOG 10.60)
		tree: () => ({ scope: { kind: 'chapter', toon, number: chapterNumber }, title: m['creator.publish_tree_title']({ name: fileLabel(`chapter:${toon}:${chapterNumber}`) }) }),
		beforeSave: () => {
			// the chapter file's (deprecated) status mirrors the season, its old `published` flag goes
			syncChapter(chapter);
			// a cast saved from here keeps the toon's star in every season
			if (castTouched || cast.dirty) castWithStar(cast, registry, toon, [SEASON]);
		},
		afterSave: () => refreshImages(),
		errorCount: () => errorCount
	});

	/** the edited document: the chapter (its groups inline – the scene documents went 2026-10-19) */
	const activeDoc = $derived<ContentDoc | null>(chapter);
	/** undo / redo act on the chapter */
	const undoDoc = $derived<ContentDoc | null>(chapter.canUndo ? chapter : null);
	const redoDoc = $derived<ContentDoc | null>(chapter.canRedo ? chapter : null);
	/**
	 * undo / redo may put the pieces back in another order (a Layers reorder) or take one away: the selected piece follows
	 * its id (gone: let go) – the text boxes / items keep their indices (their order is `layer`)
	 */
	function keepPiece(step: () => void) {
		// the list as it was: undo / redo replace the document, they never change it in place
		const before = view.selection?.kind === 'piece' ? activeSlice?.pieces : undefined;
		step();
		if (before) view.selection = followPieces(view.selection, piecesMoved(before, activeSlice?.pieces));
	}
	function undoLast() {
		const d = undoDoc;
		if (!d) return;
		keepPiece(() => d.undo());
	}
	const redoLast = () => keepPiece(() => redoDoc?.redo());
	const groupPath = $derived<Path>(['groups', view.groupIndex]);
	const slicePath = $derived<Path>([...groupPath, 'slices', view.sliceIndex]);
	const activeGroup = $derived(activeDoc?.get(activeDoc.primary, groupPath));
	const activeSlice = $derived(view.sliceIndex >= 0 ? activeDoc?.get(activeDoc.primary, slicePath) : null);
	const sliceDialogs = $derived<any[]>(activeSlice?.dialog ?? []);
	const sliceItems = $derived<any[]>(activeSlice?.items ?? []);

	// ── Navigation: the tree is the sidebar, group + slice are the deepest crumbs, the URL mirrors the selection ──
	// (`?group=<label>&slice=<n>`, `?section=audio`; deep links and Alt+ArrowUp / crumbs work through the URL)
	// the page re-creates this component per chapter ({#key})
	// svelte-ignore state_referenced_locally
	const chapterHref = `/${toon}/chapter/${chapterNumber}`;
	const groupKey = (g: any, i: number) => g?.label ?? String(i + 1);
	const groupName = (g: any, i: number) => g?.label ?? `${m['creator.groups']()} ${i + 1}`;
	/** the ActionBar's not-live chip (E.1): the chapter, the season while it has something unpublished */
	const liveFiles = $derived([
		{ doc: chapter, label: fileLabel(chapter.entity) },
		...(!season.loading && (season.dirty || season.unpublished.length) ? [{ doc: season, label: fileLabel(season.entity) }] : [])
	]);
	function hrefFor(next: { section: Section; group?: number; slice?: number }) {
		const params = new URLSearchParams();
		if (next.section === 'audio') params.set('section', 'audio');
		if ((next.section === 'group' || next.section === 'slice') && next.group !== undefined) params.set('group', groupKey(groups[next.group], next.group));
		if (next.section === 'slice' && next.slice !== undefined && next.slice >= 0) params.set('slice', String(next.slice + 1));
		const query = params.toString();
		return query ? `${chapterHref}?${query}` : chapterHref;
	}
	usePageNav({
		sidebar: tree,
		actions: flowToggle,
		crumbEnd: phoneTop,
		fill: true,
		extra: () => {
			if (view.section !== 'group' && view.section !== 'slice') return [];
			const out = [{ href: hrefFor({ section: 'group', group: view.groupIndex }), label: groupName(groups[view.groupIndex], view.groupIndex) }];
			if (view.section === 'slice' && view.sliceIndex >= 0) out.push({ href: hrefFor({ section: 'slice', group: view.groupIndex, slice: view.sliceIndex }), label: m['creator.slice_n']({ number: view.sliceIndex + 1 }) });
			return out;
		}
	});
	/** the URL has been applied once (after loading): only then does the selection write the URL */
	let urlApplied = $state(false);
	$effect(() => {
		// URL → selection (deep links, crumbs, back / forward)
		const params = page.url.searchParams;
		if (chapter.loading) return;
		const wantSection = params.get('section');
		const wantGroup = params.get('group');
		const wantSlice = Number(params.get('slice'));
		untrack(() => {
			if (wantGroup !== null) {
				let gi = groups.findIndex((g, i) => groupKey(g, i) === wantGroup);
				if (gi < 0 && /^\d+$/.test(wantGroup)) gi = Number(wantGroup) - 1;
				if (gi >= 0 && gi < groups.length) {
					if (gi !== view.groupIndex || (view.section !== 'group' && view.section !== 'slice')) view.selectGroup(gi);
					if (wantSlice > 0) {
						if (view.section !== 'slice' || view.sliceIndex !== wantSlice - 1) view.selectSlice(wantSlice - 1);
					} else if (view.section === 'slice') {
						view.selectGroup(gi);
					}
				}
			} else if (wantSection === 'audio') {
				view.section = 'audio';
			} else if (view.section !== 'meta') {
				view.section = 'meta';
			}
			urlApplied = true;
		});
	});
	$effect(() => {
		// selection → URL (shareable; a crumb goes back to the same place)
		const href = hrefFor({ section: view.section, group: view.groupIndex, slice: view.sliceIndex });
		if (!urlApplied) return;
		untrack(() => {
			// the address bar, not page.url: replaceState (shallow routing) leaves page.url at the last navigation, so
			// Audio → Meta compared with the stale URL and never cleared ?section=audio
			if (href !== location.pathname + location.search) {
				try {
					replaceState(href, {});
				} catch {
					/* router not ready */
				}
			}
		});
	});

	// ── Problems ────────────────────────────────────────────────────────────────────────────────────────
	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		const names = characters.map((c) => c.name);
		const expressions = registryOf(registry) ? expressionIndex(registryOf(registry)) : undefined;
		const moodIds = moodsOf(moods).moods.map((x) => x.id);
		const catalogItems = itemCatalog.catalog ? itemCatalog.catalog.items.map((i) => i.id) : undefined;
		const itemRespawn = itemCatalog.catalog ? Object.fromEntries(itemCatalog.catalog.items.map((i) => [i.id, i.respawn ?? null])) : undefined;
		const presets = judgedPresets(colorations);
		for (const doc of allDocs) {
			if (doc.loading) continue;
			const data = doc.data[doc.primary];
			const ctx = {
				file: doc.entity,
				audio: chapterData?.audio,
				characters: !hasCast && names.length ? names : undefined,
				cast: hasCast ? castIndex(castOf(cast), SEASON) : undefined,
				expressions,
				moods: moodIds,
				// collage pieces: a custom preset is judged once the presets are loaded
				presets,
				// the chapter's visibility: its entry in the season (null = none)
				seasonStatus: doc === chapter ? seasonStatusOf(season, languages.source, chapterNumber) : undefined,
				// reachability: what the rest of the toon jumps to / sets
				facts,
				// jump targets: the toon / chapter / label they name exist (as far as loaded)
				targets: jumpPlaces.targets(),
				// placed items (and related items) are checked against the catalog once it is loaded
				items: catalogItems,
				itemRespawn,
				// unlockLocation: the world map's places (once loaded)
				places: worldMapPlaces?.map((l) => l.id),
				toonPlaces: worldMapPlaces?.filter((l) => l.toons?.includes(toon)).map((l) => l.id),
				regions: worldMapRegions
			};
			if (data == null) {
				if (doc.source[doc.primary] === 'missing') out.push({ level: 'error', file: doc.entity, locale: null, path: [], message: m['creator.missing']() });
				continue;
			}
			// chapter, season: one language-neutral document, checked once + its texts per language
			out.push(...(doc.texts ? validateNeutral(doc.kind, data, doc.texts, ctx) : validateFile(doc.kind, data, doc.primary, ctx)));
		}
		// item limits (BACKLOG 10.39): an item placed more often than the catalog allows, every toon together
		for (const [id, c] of placementCounts) {
			const total = c.here + c.others.reduce((sum, p) => sum + p.count, 0);
			if (total <= c.limit || !c.here) continue;
			const at = placementPath(id);
			if (!at) continue;
			const item = itemCatalog.catalog?.items.find((i) => i.id === id);
			out.push({ level: 'error', file: at.file, locale: null, path: at.path, message: m['creator.clip_limit_problem']({ item: itemName(item, id), count: total, limit: c.limit, where: placesText(c.others) }) });
		}
		if (castTouched && castOf(cast)) out.push(...validateCast(castOf(cast), { registry: registryOf(registry), languages: languages.languages, file: entities.cast(toon) }));
		// suggestions from the VN usage (info): who the scenes use but the featuring / the season cast lacks
		if (!chapter.loading && hasCast) out.push(...featuringSuggestions(chapterData?.featuring, chapterSceneCharacters(groups), castIndex(castOf(cast), SEASON), chapter.entity));
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	/** the first placement of an item in this chapter's documents (a problem's place) */
	function placementPath(id: string): { file: string; path: Path } | null {
		for (const d of sliceDocs) {
			const data = d.data[d.primary];
			if (!data) continue;
			const groupsOf: [Path, any][] = Array.isArray(data.groups) ? data.groups.map((g: any, gi: number) => [['groups', gi], g]) : [[[], data]];
			for (const [base, g] of groupsOf)
				for (const [si, s] of (g?.slices ?? []).entries())
					for (const [ii, it] of (s?.items ?? []).entries()) if (it?.item === id) return { file: d.entity, path: [...base, 'slices', si, 'items', ii] };
		}
		return null;
	}
	// accepted warnings ("Looks fine") don't count
	const warningCount = $derived(problems.filter((p) => p.level === 'warning' && !isAccepted(allDocs, p)).length);

	/** a slice whose text boxes hold `{…}`: the canvas toolbar offers the sample readers */
	const sliceHasExpressions = (slice: any) =>
		(slice?.dialog ?? []).some((d: any) => {
			const t = d?.text;
			const all = t && typeof t === 'object' && !Array.isArray(t) ? Object.values(t) : [t];
			return all.flat().some((x) => typeof x === 'string' && x.includes('{'));
		});

	function gotoProblem(p: Problem) {
		// an expression's problem: its field (once shown) focuses and selects the range (expr-text.svelte)
		if (p.expr) exprContext.focus = { key: fieldKey(p.file, p.path, p.locale), line: p.expr.line, start: p.expr.start, end: p.expr.end };
		let path = p.path;
		if (p.file === season.entity) {
			view.section = 'meta';
			return;
		}
		if (p.file !== chapter.entity) return;
		if (path[0] === 'groups' && typeof path[1] === 'number') {
			view.selectGroup(path[1]);
			path = path.slice(2);
		} else {
			view.section = path[0] === 'audio' ? 'audio' : 'meta';
			return;
		}
		// a problem in another language switches the header's text language to it
		if (p.locale && p.locale !== displayLocale && (languages.languages as string[]).includes(p.locale)) textLanguage.set(p.locale);
		if (path[0] === 'slices' && typeof path[1] === 'number') {
			view.selectSlice(path[1]);
			const rest = path.slice(2);
			if (rest[0] === 'dialog' && typeof rest[1] === 'number') view.select({ kind: 'dialog', index: rest[1] });
			else if (rest[0] === 'items' && typeof rest[1] === 'number') view.select({ kind: 'item', index: rest[1] });
			else if (rest[0] === 'pieces' && typeof rest[1] === 'number') view.select({ kind: 'piece', index: rest[1] });
			else if (rest[0] === 'audio') view.inspectorTab = 'audio';
			else if (rest[0] === 'events') view.inspectorTab = 'events';
			else view.inspectorTab = 'slice';
		}
	}

	// ── Flow panel (owner 2026-10-06, read-only): a box opens its place in the editor – navigation only ──────────
	/** a group's name in the graph (the tree's) and a text in the header's language (a choice's label) */
	const flowName = (g: any, i: number) => groupName(groups[i] ?? g, i);
	const flowText = (v: unknown): string => {
		if (typeof v === 'string') return v;
		if (!v || typeof v !== 'object') return '';
		const map = v as Record<string, unknown>;
		const pick = [displayLocale, languages.source, ...Object.keys(map)].map((l) => map[l]).find((t) => typeof t === 'string' && t.trim());
		return typeof pick === 'string' ? pick : '';
	};
	/** a character key of the graph (`boy`, `catpope·cat`) as its name in the header's language */
	const flowWho = (key: string): string => {
		const [id, persona] = key.split('·');
		return shownCharacterName(registryIndex.get(id), persona) || key;
	};
	/** the chapter the follow-up bar leads to after the last group: the season's next one (the Flow's next-chapter picture) */
	const flowNextChapter = $derived.by(() => {
		const items: any[] = (season.get(season.primary, ['sequence']) as any[] | undefined) ?? [];
		const numbers = items.filter((i) => (i?.type ?? 'chapter') === 'chapter' && Number.isFinite(Number(i?.number))).map((i) => Number(i.number));
		const at = numbers.indexOf(chapterNumber);
		return at >= 0 ? numbers[at + 1] : undefined;
	});
	async function openFlowNode(node: FlowNode) {
		if (!node.path) return;
		gotoProblem({ level: 'info', file: chapter.entity, locale: null, path: node.path, message: '' });
		// a script's event: scroll its row into view (the group's VN list)
		const [, , list, index] = node.path;
		if (list === 'events' && typeof index === 'number') {
			await tick();
			document.querySelector(`[data-testid="vn-events"] [data-testid="event-${index}"]`)?.scrollIntoView({ block: 'center' });
		}
	}

	// ── Editing actions (lib/editor/slice-edit.ts) ────────────────────────────────────────────────────────────
	/** the chapter's group labels: a jump's group and the random list (CUTOVER 10.27) */
	const groupLabels = $derived(groups.map((g, gi) => groupName(g, gi)).filter(Boolean));
	// the places jumps lead to (the target editor's toon → chapter → label; validation checks targets against them)
	// svelte-ignore state_referenced_locally
	const jumpPlaces = setJumpPlaces(new JumpPlaces(toon, chapterNumber, () => groups.map((g) => g?.label).filter((l): l is string => typeof l === 'string' && !!l)));
	$effect(() => {
		jumpPlaces.toons = toonInfos.length ? toonInfos.map((t) => ({ id: t.id, name: t.name })) : null;
	});
	// load what the chapter's targets name (other chapters' / toons' labels) – once per target, debounced while editing
	$effect(() => {
		const docs = sliceDocs.map((d) => $state.snapshot(d.data[d.primary]));
		const timer = setTimeout(() => jumpPlaces.prefetch(docs), 500);
		return () => clearTimeout(timer);
	});
	/** every text box id of the chapter: a new box gets a fresh one (owner 2026-10-06: unique in the chapter) */
	const chapterDialogIds = () =>
		new Set<string>(groups.flatMap((g: any) => (g?.slices ?? []).flatMap((s: any) => (s?.dialog ?? []).map((d: any) => d?.id).filter((id: unknown) => typeof id === 'string'))));
	const sliceTarget = (): edits.SliceTarget | null =>
		activeDoc && view.sliceIndex >= 0 ? { doc: activeDoc, slicePath, chapterNumber, sliceIndex: view.sliceIndex, slice: activeSlice, taken: chapterDialogIds } : null;

	// linked groups (owner 2026-10-06): a group whose script can jump away and that continues – the next group needs a
	// label, else nothing leads back to it (a reminder with one click, next to the script)
	const nextGroup = $derived(groups[view.groupIndex + 1]);
	const needsNextLabel = $derived(!!activeGroup?.continues && scriptJumps(activeGroup) && !!nextGroup && !nextGroup.label);
	function labelNextGroup() {
		const index = view.groupIndex + 1;
		const taken = new Set(groups.map((g: any) => g?.label).filter(Boolean));
		let label = `scene-${index + 1}`;
		for (let n = 2; taken.has(label); n++) label = `scene-${index + 1}-${n}`;
		chapter.edit((file) => void (file.groups[index].label = label));
	}
	/** a new text with a preset's look (default speech; a drawn one keeps the drawn rect); the cursor goes to its text */
	function addDialog(rect?: { x: number; y: number; width: number; height?: number }, preset?: TextPreset) {
		view.pickingPreset = false;
		const t = sliceTarget();
		if (!t) return;
		view.select(edits.addDialog(t, rect, preset));
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>('[data-testid="dialog-inspector"] textarea')?.focus());
	}
	function createBoxForEvent(index: number, eventsPath?: Path) {
		const t = sliceTarget();
		if (!t) return;
		// the new box is selected (the text box tab opens) with its text ready to type
		view.select(edits.createBoxForEvent(t, index, eventsPath));
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>('[data-testid="dialog-inspector"] textarea')?.focus());
	}
	function addSliceItem(itemId: string) {
		const spot = view.itemSpot;
		view.itemSpot = null;
		const t = sliceTarget();
		if (!t || !itemId) return;
		// the item's placement limit (BACKLOG 10.39): refused with a toast
		const refused = placementLimit(new Map([[itemId, 1]]));
		if (refused) return void toast(refused.message, { type: 'error', timeout: 0 });
		view.select(edits.addSliceItem(t, itemId, spot));
	}
	/** a picked collage piece at the piece tool's spot (research/COLLAGE.md §3: a click = centred there, a drag = its rect) */
	function addPiece(picked: PickedPiece) {
		const place = view.pieceSpot ?? piecePlace(null);
		view.pieceSpot = null;
		const t = sliceTarget();
		if (t) view.select(edits.addPiece(t, picked, place));
	}
	function addSliceEvent(rect: { x: number; y: number; width: number; height: number }) {
		const t = sliceTarget();
		if (t) view.select(edits.addSliceEvent(t, rect));
	}
	function addSampleArea(rect: { x: number; y: number; width: number; height: number }) {
		const t = sliceTarget();
		if (!t) return;
		const next = edits.addSampleArea(t, rect, chapterData?.audio?.sampleLibrary ?? []);
		if (next) view.select(next);
		else session.notify('The chapter has no sample library yet (Audio in the tree).');
	}
	/** what a tool makes of a click / a drawn rect on the canvas (a click: width 0) */
	const CREATE: Record<Tool, (rect: { x: number; y: number; width: number; height: number }) => void> = {
		// a tap (no rect – the phone, E3): the picked preset at its size, centred on the tap
		text: (rect) => {
			if (rect.width >= 2) return addDialog(rect);
			const preset = TEXT_PRESETS.find((p) => p.id === view.textPreset) ?? TEXT_PRESETS[0];
			const w = preset.rect.width ?? 70;
			const h = preset.rect.height ?? 10;
			const x = Math.max(0, Math.min(100 - w, rect.x - w / 2));
			addDialog({ x: Math.round(x * 100) / 100, y: Math.round((rect.y - h / 2) * 100) / 100, width: w, ...(preset.rect.height !== undefined ? { height: h } : {}) }, preset);
		},
		item: (rect) => view.pickItem({ x: rect.x, y: rect.y }),
		piece: (rect) => view.pickPiece(rect),
		sample: (rect) => addSampleArea(rect),
		event: (rect) => addSliceEvent(rect)
	};
	function removeSelected() {
		const t = sliceTarget();
		if (!t || !view.selection) return;
		edits.removeSelected(t, view.selection);
		view.selection = null;
	}

	// ── phones (UI-CONCEPTS E.10, owner 2026-10-11, like CapCut) ──────────────────────────────────────────────────
	/** the inspector sheet, variant C: closed it is its tab row only; a tab opens it there, the open tab closes it */
	let sheetOpen = $state(false);
	/**
	 * the tab row drags the sheet (owner 2026-10-11): up = taller / open, down = lower, let go low = closed; a drag is no
	 * tap. The drag starts from the height the sheet is drawn at (open: it may fill the room a short slice leaves; closed:
	 * the tab row) and then sets exactly the dragged height (`sheet-sized`), so the sheet follows the finger (owner
	 * 2026-10-18: the sheet "jumps" and "can't be moved"). Move / up are followed on the window: a pointer that leaves the
	 * row before the drag counts (a fast flick, a mouse) is still the drag; the row captures it once it moves.
	 */
	let tabDrag: { id: number; y: number; start: number; moved: boolean; row: HTMLElement } | null = null;
	let swallowTap = false;
	function tabsDown(event: PointerEvent) {
		if (tabDrag || (event.pointerType === 'mouse' && event.button !== 0)) return;
		swallowTap = false; // a new press: its tap counts (only the click a drag ends with is swallowed)
		const row = event.currentTarget as HTMLElement;
		const drawn = (sheetOpen ? document.getElementById(INSPECTOR) : row)?.getBoundingClientRect().height;
		tabDrag = { id: event.pointerId, y: event.clientY, start: drawn ? drawn / rootRem() : 2.75, moved: false, row };
		window.addEventListener('pointermove', tabsMove);
		window.addEventListener('pointerup', tabsUp);
		window.addEventListener('pointercancel', tabsUp);
	}
	function closeSheet() {
		sheetOpen = false;
		sheetRem = null;
	}
	function tabsMove(event: PointerEvent) {
		if (!tabDrag || tabDrag.id !== event.pointerId) return;
		const dy = event.clientY - tabDrag.y;
		if (!tabDrag.moved && Math.abs(dy) < 8) return;
		if (!tabDrag.moved) try {
			tabDrag.row.setPointerCapture(event.pointerId);
		} catch {
			/* a pointer that is gone */
		}
		tabDrag.moved = true;
		sheetOpen = true;
		setSheet(tabDrag.start - dy / rootRem());
	}
	function tabsUp(event: PointerEvent) {
		if (!tabDrag || tabDrag.id !== event.pointerId) return;
		const moved = tabDrag.moved;
		tabDrag = null;
		window.removeEventListener('pointermove', tabsMove);
		window.removeEventListener('pointerup', tabsUp);
		window.removeEventListener('pointercancel', tabsUp);
		if (!moved) return;
		swallowTap = true;
		setTimeout(() => (swallowTap = false), 300);
		if ((sheetRem ?? 99) < 6) closeSheet();
	}
	$effect(() => () => {
		window.removeEventListener('pointermove', tabsMove);
		window.removeEventListener('pointerup', tabsUp);
		window.removeEventListener('pointercancel', tabsUp);
	});
	function pickSheetTab(id: InspectorTab) {
		if (swallowTap) return;
		if (view.inspectorTab === id && sheetOpen) closeSheet();
		else {
			view.setTab(id);
			sheetOpen = true;
		}
	}
	/** the phone's long-press context (E3): on a box of the strip or a Layers row – the selection is already made */
	let longMenu = $state<DOMRect | null>(null);
	/** Edit (the pencil): a text box in place, anything else its inspector */
	function editSelected() {
		const sel = view.selection;
		if (!sel) return;
		if (sel.kind === 'dialog') return canvas?.editText(sel.index);
		pickSheetTab(sel.kind === 'event' ? 'events' : sel.kind === 'sample' ? 'audio' : 'layers');
	}
	/** copy / cut / paste on the slice (the live region), as the clipboard menu */
	async function clip(action: 'copy' | 'cut' | 'paste' | 'duplicate') {
		liveRegion.set(SLICE_REGION);
		if (action === 'paste') return void (await clipboard.pasteFromMenu(false));
		await clipboard.copyFromMenu(action === 'cut');
		if (action === 'duplicate') await clipboard.pasteFromMenu(false);
	}
	/**
	 * the bar (UI-CONCEPTS E.10; every width since 2026-10-12, owner: "phone grouping"): Save · History (undo, redo,
	 * versions) · Publish · More; on a desktop Problems and the slice stepper lead the row (More drops them)
	 */
	const phoneWidth = new MediaQuery('max-width: 767px', false);
	const problemsItem = $derived<PhoneBarItem>({ id: 'problems', icon: 'alert-triangle', label: m['creator.problems'](), badge: errorCount + warningCount || undefined, active: open.problems, run: () => togglePanel('problems'), testid: 'problems-toggle' });
	const stepping = $derived(view.section === 'slice' && !!activeGroup?.slices?.length);
	const phoneItems = $derived<PhoneBarItem[]>([
		...(phoneWidth.current ? [] : [problemsItem, ...(stepping ? [{ id: 'slices', label: m['creator.slice_navigation'](), content: deskSliceNav }] : [])]),
		{ id: 'save', icon: 'save', label: anySaving ? m['creator.saving']() : m['creator.save_short'](), primary: anyDirty, badge: anyDirty ? '•' : undefined, disabled: !anyDirty || anySaving, run: () => session.saveDraft(), testid: 'save-draft' },
		{
			id: 'history',
			icon: 'clock',
			label: m['creator.history'](),
			items: [
				{ id: 'undo', icon: 'rotate-ccw', label: m['creator.undo'](), disabled: !undoDoc, run: undoLast, stay: true, testid: 'undo' },
				{ id: 'redo', icon: 'rotate-cw', label: m['creator.redo'](), disabled: !redoDoc, run: redoLast, stay: true, testid: 'redo' },
				{ id: 'versions', icon: 'list', label: m['creator.bar_versions'](), active: open.history, run: () => togglePanel('history'), testid: 'history-versions' }
			]
		},
		// a dot while saved drafts are not live yet (owner 2026-10-11: the state on Save / Publish, no status chip in More)
		{ id: 'publish', icon: 'upload-cloud', label: publishLabel(toon), disabled: anySaving, badge: liveFiles.some((f) => f.doc.unpublished.length) ? '•' : undefined, run: () => session.publishAll(), testid: 'publish' },
		{
			id: 'more',
			icon: 'more-horizontal',
			label: m['creator.more'](),
			badge: errorCount || undefined,
			items: [
				...(phoneWidth.current ? [problemsItem] : []),
				{ id: 'paste', icon: 'clipboard', label: m['creator.clip_paste'](), run: () => clip('paste') },
				...(phoneWidth.current && stepping && activeGroup
					? [
							{ id: 'prev-slice', icon: 'chevron-left', label: m['creator.bar_prev_slice'](), disabled: view.sliceIndex <= 0, run: () => view.selectSlice(view.sliceIndex - 1), stay: true },
							{ id: 'next-slice', icon: 'chevron-right', label: m['creator.bar_next_slice'](), disabled: view.sliceIndex >= activeGroup.slices.length - 1, run: () => view.selectSlice(view.sliceIndex + 1), stay: true }
						]
					: [])
			]
		}
	]);
	// ── phones: picking slices in the group's grid and groups in the chapter's list (owner 2026-10-12: "das Zurück unten
	// in der Auswahl-Leiste … brutal einführen" – chapter editor slices / groups). A long press picks one, then taps add /
	// remove; a tap without picks still opens it (BACKLOG 10.50 h). The picks are a selection: the bar shows their row
	// (copy / cut / paste through the same handlers as the tree's lists), ‹ drops them ──
	const PHONE_SLICES = 'phone-slices';
	const PHONE_GROUPS = 'phone-groups';
	let phonePicks = $state<number[]>([]);
	let phoneGroupPicks = $state<number[]>([]);
	$effect(() => {
		void view.groupIndex;
		void view.section;
		untrack(() => {
			phonePicks = [];
			phoneGroupPicks = [];
		});
	});
	const phoneSliceClip = slicesClipHandler({
		doc: () => activeDoc!,
		groupPath: () => groupPath,
		slices: () => activeGroup?.slices ?? [],
		picked: () => phonePicks,
		pick: (next) => (phonePicks = next),
		onselect: () => {}
	});
	useClipHandler(() => (view.section === 'group' && activeDoc ? PHONE_SLICES : null), phoneSliceClip);
	const phoneGroupClip = groupsClipHandler({
		chapter: () => chapter,
		groups: () => groups,
		chapterNumber: () => chapterNumber,
		picked: () => phoneGroupPicks,
		pick: (next) => (phoneGroupPicks = next),
		onremove: () => {},
		onpaste: () => {}
	});
	useClipHandler(() => (view.section === 'meta' ? PHONE_GROUPS : null), phoneGroupClip);
	/** a long press (450 ms without moving) picks; the tap that ends it does nothing else */
	let press: { timer: ReturnType<typeof setTimeout>; x: number; y: number; fired: boolean } | null = null;
	function pressStart(event: PointerEvent, fire: () => void) {
		press = { x: event.clientX, y: event.clientY, fired: false, timer: setTimeout(() => press && ((press.fired = true), fire()), 450) };
	}
	function pressMove(event: PointerEvent) {
		if (press && !press.fired && Math.hypot(event.clientX - press.x, event.clientY - press.y) > 8) pressEnd();
	}
	function pressEnd() {
		if (press && !press.fired) {
			clearTimeout(press.timer);
			press = null;
		}
	}
	/** a tap: false when it only ended a long press */
	function pressTap(): boolean {
		const ended = !!press?.fired;
		press = null;
		return !ended;
	}
	const addPick = (picks: number[], index: number) => (picks.includes(index) ? picks : [...picks, index].sort((a, b) => a - b));
	const togglePick = (picks: number[], index: number) => (picks.includes(index) ? picks.filter((i) => i !== index) : addPick(picks, index));
	function sliceTap(index: number) {
		if (!pressTap()) return;
		if (!phonePicks.length) return view.selectSlice(index);
		phonePicks = togglePick(phonePicks, index);
	}
	function groupTap(index: number) {
		if (!pressTap()) return;
		if (!phoneGroupPicks.length) return view.selectGroup(index);
		phoneGroupPicks = togglePick(phoneGroupPicks, index);
	}
	async function pickedClip(region: string, action: 'copy' | 'cut' | 'duplicate' | 'delete', what: string) {
		liveRegion.set(region);
		if (action === 'delete') {
			if (await confirmRemove(m['creator.remove_confirm']({ what }))) clipboard.handler?.remove();
			return;
		}
		if (action === 'duplicate') {
			const copied = clipboard.copy();
			if (copied) clipboard.paste(copied);
			return;
		}
		await clipboard.copyFromMenu(action === 'cut');
	}
	/** the bar's row for picked slices / groups */
	function picksRow(kind: 'slices' | 'groups', picks: number[], clear: () => void, open: (index: number) => void) {
		const region = kind === 'slices' ? PHONE_SLICES : PHONE_GROUPS;
		const what = kind === 'slices' ? m['creator.clip_what_slices']({ count: picks.length }) : m['creator.clip_what_groups']({ count: picks.length });
		const run = (action: 'copy' | 'cut' | 'duplicate' | 'delete') => () => pickedClip(region, action, what);
		return {
			label: what,
			onback: clear,
			items: [
				...(picks.length === 1 ? [{ id: 'open', icon: 'edit-2', label: m['creator.bar_edit'](), run: () => open(picks[0]), testid: `phone-${kind}-open` }] : []),
				{ id: 'duplicate', icon: 'copy', label: m['creator.bar_duplicate'](), run: run('duplicate'), testid: `phone-${kind}-duplicate` },
				{ id: 'copy', icon: 'clipboard', label: m['creator.clip_copy'](), run: run('copy'), testid: `phone-${kind}-copy` },
				{ id: 'cut', icon: 'scissors', label: m['creator.clip_cut_action'](), run: run('cut'), testid: `phone-${kind}-cut` },
				{ id: 'delete', icon: 'trash-2', label: m['creator.bar_delete'](), run: run('delete'), testid: `phone-${kind}-delete` }
			] satisfies PhoneBarItem[]
		};
	}

	/** the selected text box / item / area: its own actions (‹ deselects) */
	const phoneSelection = $derived.by(() => {
		if (view.section === 'group' && phonePicks.length) return picksRow('slices', phonePicks, () => (phonePicks = []), (i) => view.selectSlice(i));
		if (view.section === 'meta' && phoneGroupPicks.length) return picksRow('groups', phoneGroupPicks, () => (phoneGroupPicks = []), (i) => view.selectGroup(i));
		const sel = view.selection;
		if (!sel || view.section !== 'slice') return null;
		const tab: InspectorTab = sel.kind === 'event' ? 'events' : sel.kind === 'sample' ? 'audio' : 'layers';
		const label = { dialog: m['creator.bar_sel_text'](), item: m['creator.bar_sel_item'](), piece: m['creator.bar_sel_piece'](), event: m['creator.bar_sel_event'](), sample: m['creator.bar_sel_sample']() }[sel.kind];
		return {
			label,
			onback: () => (view.selection = null),
			items: [
				{ id: 'edit', icon: 'edit-2', label: m['creator.bar_edit'](), active: sheetOpen && view.inspectorTab === tab, run: () => ((sheetOpen = true), view.setTab(tab)), testid: 'phone-sel-edit' },
				{ id: 'duplicate', icon: 'copy', label: m['creator.bar_duplicate'](), run: () => clip('duplicate'), testid: 'phone-sel-duplicate' },
				{ id: 'copy', icon: 'clipboard', label: m['creator.clip_copy'](), run: () => clip('copy') },
				{ id: 'cut', icon: 'scissors', label: m['creator.clip_cut_action'](), run: () => clip('cut') },
				{ id: 'delete', icon: 'trash-2', label: m['creator.bar_delete'](), run: removeSelected, testid: 'phone-sel-delete' }
			] satisfies PhoneBarItem[]
		};
	});

	// ── Guards, autosave, keyboard ──────────────────────────────────────────────────────────────────────
	// unsaved changes: leaving asks in-page; lib/editor/ask.ts
	beforeNavigate(leaveGuard(() => anyDirty));

	$effect(() => {
		const docs = allDocs;
		for (const d of docs) void JSON.stringify(d.data);
		const timer = setTimeout(() => docs.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	/** the selection region of the slice canvas + its inspector (Layers, audio samples, slice events: one selection) */
	const SLICE_REGION = 'slice';

	// copy / cut / paste (BACKLOG 10.39, lib/editor/clipboard.svelte.ts): the DOM events act on the live region's handler;
	// the canvas region copies its selection and pastes into the open slice
	$effect(() => clipboardEvents());
	// ── item limits (BACKLOG 10.39 step 7, ITEMS.md "Item limits"): a catalog item's `placements` count over every toon –
	// this chapter's documents live, every other chapter from the API (GET /api/catalog/items/<id>/placements,
	// loaded once per limited item); over the limit = a Problems error, a paste that would go over it is refused
	let placedElsewhere = $state<Record<string, { key: string; count: number }[]>>({});
	$effect(() => {
		const items = itemCatalog.catalog?.items ?? [];
		for (const it of items) {
			if (!it.placements || untrack(() => it.id in placedElsewhere)) continue;
			placedElsewhere = { ...untrack(() => placedElsewhere), [it.id]: [] };
			api.get<{ placements: { key: string; count: number }[] }>(`/catalog/items/${encodeURIComponent(it.id)}/placements`)
				.then((r) => (placedElsewhere = { ...placedElsewhere, [it.id]: r.placements ?? [] }))
				.catch(() => {});
		}
	});
	/** per limited item: its limit, the placements in this chapter's documents and elsewhere (other chapters) */
	const placementCounts = $derived.by(() => {
		const out = new Map<string, { limit: number; here: number; others: { key: string; count: number }[] }>();
		const own = new Set(sliceDocs.map((d) => d.entity));
		const here = new Map<string, number>();
		for (const d of sliceDocs) if (!d.loading) for (const [id, n] of countPlacements(d.data[d.primary])) here.set(id, (here.get(id) ?? 0) + n);
		for (const it of itemCatalog.catalog?.items ?? []) {
			if (!it.placements) continue;
			out.set(it.id, { limit: it.placements, here: here.get(it.id) ?? 0, others: (placedElsewhere[it.id] ?? []).filter((p) => !own.has(p.key)) });
		}
		return out;
	});
	const placesText = (others: { key: string; count: number }[]) =>
		[m['creator.clip_limit_here'](), ...others.map((p) => `${fileLabel(p.key)} (${p.count})`)].join(', ');
	/** a paste that would place catalog items over their limit: the refusal (a toast); null = fine */
	const placementLimit = (adds: Map<string, number>): ClipResult | null => {
		for (const [id, n] of adds) {
			const c = placementCounts.get(id);
			if (!c) continue;
			const total = c.here + n + c.others.reduce((sum, p) => sum + p.count, 0);
			if (total > c.limit) {
				const item = itemCatalog.catalog?.items.find((i) => i.id === id);
				return { message: m['creator.clip_limit']({ item: itemName(item, id), limit: c.limit, count: total, where: placesText(c.others) }), error: true };
			}
		}
		return null;
	};
	useClipHandler(
		() => (view.section === 'slice' ? SLICE_REGION : null),
		// svelte-ignore state_referenced_locally
		sliceClipHandler({ view, target: sliceTarget, toon, chapterNumber, chapter, placementLimit })
	);
	// the slice list, the tree and the VN events: their copy / paste share the chapter, its box ids, labels, the limits
	$effect(() =>
		setClipContext({
			chapter,
			toon,
			chapterNumber,
			boxIds: chapterDialogIds,
			groupLabels: () => groups.map((g: any) => g?.label).filter((l: unknown): l is string => typeof l === 'string' && !!l),
			placementLimit
		})
	);

	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		const key = event.key.toLowerCase();
		// Ctrl+S saves (no note dialog); undo / redo only outside a field – in one, they are the browser's (owner
		// 2026-10-03, COMPONENTS.md 4.4.1); lib/editor/editor-keys.ts
		if (editorKeys(event, { save: () => session.saveAll(), undo: undoLast, redo: redoLast })) return;
		if (isField(event.target) || view.section !== 'slice') return;
		// the keys act on the LIVE region only (DESIGN.md "Selection", BACKLOG 10.40): the canvas + its inspector. A
		// Delete in the slice list or the tree no longer removes the canvas's text box.
		if (!liveRegion.is(SLICE_REGION)) return;
		// one grid step (snap off: 1 px), Shift × 10
		const step = (view.grid || 1) * (event.shiftKey ? 10 : 1);
		const arrows: Record<string, [number, number]> = { arrowleft: [-step, 0], arrowright: [step, 0], arrowup: [0, -step], arrowdown: [0, step] };
		if (arrows[key] && view.selection) {
			event.preventDefault();
			canvas?.nudge(...arrows[key]);
		} else if ((key === 'delete' || key === 'backspace') && !mod) {
			// Delete / Backspace remove the selected text, item, sample area or event area (Ctrl+X = cut: the clipboard)
			if (!view.selection) return;
			event.preventDefault();
			removeSelected();
		} else if (key === 'escape') {
			view.selection = null;
		}
	}

</script>

<!-- the open group: its form (label, layout, end …) and its script – the group section, and the slice editor's Group tab
     (the strip editor, TEXT-ACROSS-SLICES.md §4.1: the group and its slices on one screen) -->
{#snippet groupBody()}
	{#if activeDoc}
		<GroupPanel doc={activeDoc} {groupPath} {toon} {chapterNumber} {groupLabels} last={view.groupIndex === groups.length - 1} />
		<h3 class="sub">{m['creator.vn']()}</h3>
		{#if needsNextLabel}
			<div class="sc-bar is-note" data-testid="next-label-hint">
				<span>{m['creator.next_label_hint']()}</span>
				<button class="secondary" onclick={labelNextGroup} data-testid="next-label-add">{m['creator.next_label_add']()}</button>
			</div>
		{/if}
		{#if activeGroup?.events}
			<VnEvents doc={activeDoc} eventsPath={[...groupPath, 'events']} {characters} {choices} dialogs={edits.boxChoices(activeGroup?.slices, [displayLocale, languages.source])} {groupLabels} markOf={(i) => markAt('groups', view.groupIndex, 'events', i)} />
		{:else}
			<p><button class="secondary" onclick={() => activeDoc?.edit((f) => { const g = getAt(f, groupPath); if (g) g.events = []; })}>+ {m['creator.vn_script_add']()}</button></p>
		{/if}
	{/if}
{/snippet}

<svelte:window onkeydown={onKeydown} />
<!-- right-click / long-press in a copy region: Copy · Cut · Paste · Paste settings (BACKLOG 10.39 step 8) -->
<ClipMenu />

<!-- the Flow panel opens where the Ask panel does: a round button in the top bar (owner 2026-10-06); the Reader next to it
     (BACKLOG 10.49) -->
<!-- phones, the strip editor (TEXT-ACROSS-SLICES.md §4.1 E3): the crumb row is the top bar – ⚠ · Save · Publish · •••
     (versions, paste, snap, seams); the bottom row is the kinds (PhoneKinds) -->
{#snippet phoneTop()}
	{#if phoneWidth.current && view.section === 'slice' && activeSlice}
		<span class="phone-top" data-testid="phone-top">
			<button type="button" class="sq" title={m['creator.problems']()} aria-label={m['creator.problems']()} aria-pressed={open.problems} onclick={() => togglePanel('problems')} data-testid="problems-toggle"><Icon name="alert-triangle" />{#if errorCount + warningCount}<span class="badge">{errorCount + warningCount}</span>{/if}</button>
			<button type="button" class="sq" title={anySaving ? m['creator.saving']() : m['creator.save_short']()} aria-label={m['creator.save_short']()} disabled={!anyDirty || anySaving} onclick={() => session.saveDraft()} data-testid="save-draft"><Icon name="save" />{#if anyDirty}<span class="dot">•</span>{/if}</button>
			<button type="button" class="sq" title={publishLabel(toon)} aria-label={publishLabel(toon)} disabled={anySaving} onclick={() => session.publishAll()} data-testid="publish"><Icon name="upload-cloud" />{#if liveFiles.some((f) => f.doc.unpublished.length)}<span class="dot">•</span>{/if}</button>
			<Menu label={m['creator.more']()} align="end">
				{#snippet trigger()}<span data-testid="phone-bar-more"><Icon name="more-horizontal" label={m['creator.more']()} /></span>{/snippet}
				<button type="button" onclick={() => togglePanel('history')} data-testid="history-versions">{m['creator.bar_versions']()}</button>
				<button type="button" onclick={() => clip('paste')} data-testid="paste">{m['creator.clip_paste']()}</button>
				<button type="button" aria-pressed={view.snap} onclick={() => (view.snap = !view.snap)} data-testid="phone-snap">{m['creator.canvas_snap']()}</button>
				<button type="button" aria-pressed={view.seams} onclick={() => (view.seams = !view.seams)} data-testid="phone-seams">{m['creator.canvas_seams']()}</button>
			</Menu>
		</span>
	{/if}
{/snippet}

{#snippet flowToggle()}
	<button type="button" class="sc-round" aria-pressed={open.reader} title={m['creator.reader_open']()} aria-label={m['creator.reader_open']()} onclick={() => togglePanel('reader')} data-testid="reader-toggle"><Icon name="smartphone" /></button>
	<button type="button" class="sc-round" aria-pressed={open.flow} title={m['creator.flow_open']()} aria-label={m['creator.flow_open']()} onclick={() => togglePanel('flow')} data-testid="flow-toggle"><Icon name="git-merge" /></button>
{/snippet}

{#snippet tree()}
	<ChapterTree {view} {chapter} {groups} {groupName} {activeDoc} {groupPath} {chapterNumber} {markAt} />
{/snippet}

<div class="editor">

	<!-- Bottom action bar (PLATFORM.md "Bottom action bar"): navigation + validation summary · tools · save / publish -->
	<!-- prev / next slice: in the bar's end from 768px (always at hand), in "More" on phones (the end holds only save) -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet phone()}
			{#if phoneWidth.current && view.section === 'slice' && activeSlice}
				<PhoneKinds {view} canUndo={!!undoDoc} canRedo={!!redoDoc} onundo={undoLast} onredo={redoLast} oninspector={pickSheetTab} />
			{:else}
				<PhoneBar items={phoneItems} selection={phoneSelection} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
			{/if}
		{/snippet}
	</ActionBar>

	{#if longMenu}
		<LongPressMenu at={longMenu} oncopy={() => clip('copy')} oncut={() => clip('cut')} ondelete={removeSelected} onedit={editSelected} onclose={() => (longMenu = null)} />
	{/if}
	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		<DocNotices {session} docs={noticeDocs} />

		<!-- the slice canvas + its inspector = ONE selection region (live on load; the keys act on it) -->
		<div class="columns" use:region={view.section === 'slice' ? { name: SLICE_REGION, initial: true } : null} class:has-inspector={view.section === 'slice'} class:inspector-max={inspectorMax && view.section === 'slice'} class:sheet-closed={!sheetOpen} class:sheet-sized={sheetOpen && sheetRem !== null} bind:this={columnsEl} style:--inspector-width={inspectorSize.css} style:--sheet-height={sheetRem === null ? null : `${sheetRem}rem`}>
			<!-- Main -->
			<main class="main" class:fit={view.section === 'slice' && !!activeSlice}>
				{#if view.section === 'meta'}
					<!-- phones (BACKLOG 10.50 h): the chapter leads with its groups – the tree is in the drawer there -->
					<nav class="phone-only phone-groups" aria-label={m['creator.groups']()} data-testid="phone-groups">
						<p class="sc-nav-head">{m['creator.groups']()}</p>
						{#each groups as g, gi (gi)}
							<button
								type="button"
								class="sc-nav-item phone-group"
								class:sc-selected={phoneGroupPicks.includes(gi)}
								aria-pressed={phoneGroupPicks.length ? phoneGroupPicks.includes(gi) : undefined}
								onpointerdown={(e) => pressStart(e, () => (phoneGroupPicks = addPick(phoneGroupPicks, gi)))}
								onpointermove={pressMove}
								onpointerup={pressEnd}
								onpointercancel={pressEnd}
								oncontextmenu={(e) => e.preventDefault()}
								onclick={() => groupTap(gi)}
								data-testid="phone-group-{gi}"
							>
								<span class="phone-name">{groupName(groups[gi], gi)}</span>
								<span class="phone-hint">{#if g?.events}<Icon name="zap" size="0.875rem" label={m['creator.events']()} />{/if}{g?.slices?.length ?? 0}</span>
							</button>
						{/each}
					</nav>
					{#if legacyTexts}
						<div class="sc-bar is-note" data-testid="legacy-texts">
							<span>{m['creator.legacy_texts']({ count: legacyTexts })}</span>
							<button class="secondary" onclick={convertLegacyTexts} data-testid="convert-presets">{m['creator.legacy_texts_convert']()}</button>
						</div>
					{/if}
					<MetaPanel doc={chapter} {season} number={chapterNumber} {images}>
						{#snippet featuring()}
							{#if !cast.loading && !registry.loading}
								<FeaturingField
									doc={chapter}
									cast={seasonCast}
									registry={registryOf(registry)}
									{star}
									groups={() => groups}
									addToCast={(id) => ensureCast(id, REGULAR_PERSONA)}
								/>
							{/if}
							<!-- the chapter's related items (catalog ids, the same in every language); the reader does not show them yet -->
							<RelatedItemsField doc={chapter} hint={m['creator.related_items_hint_chapter']()} testid="chapter-related-items" />
						{/snippet}
					</MetaPanel>
				{:else if view.section === 'audio'}
					<AudioPanel doc={chapter} {season} number={chapterNumber} />
				{:else if view.section === 'group'}
					{#if activeDoc && !activeDoc.loading}
						<!-- phones (BACKLOG 10.50 h): the group's slices first – a tap opens the slice editor -->
						{#if activeGroup?.slices?.length}
							<nav class="phone-only phone-slices" aria-label={m['creator.slices']()} data-testid="phone-slices">
								{#each activeGroup.slices as s, si (si)}
									<button
										type="button"
										class="phone-slice"
										class:sc-selected={phonePicks.includes(si)}
										aria-pressed={phonePicks.length ? phonePicks.includes(si) : undefined}
										onpointerdown={(e) => pressStart(e, () => (phonePicks = addPick(phonePicks, si)))}
										onpointermove={pressMove}
										onpointerup={pressEnd}
										onpointercancel={pressEnd}
										oncontextmenu={(e) => e.preventDefault()}
										onclick={() => sliceTap(si)} aria-label={m['creator.slice_n']({ number: si + 1 })} title={m['creator.slice_n']({ number: si + 1 })} data-control-exempt="slice thumbnail" data-testid="phone-slice-{si}">
										<img src={toonUrl(activeDoc.toon, s?.image)} alt="" loading="lazy" draggable="false" />
										<span>{si + 1}</span>
									</button>
								{/each}
							</nav>
						{/if}
						{@render groupBody()}
					{:else if activeDoc?.error}
						<p class="error">{activeDoc.error}</p>
					{:else}
						<p>{m['creator.loading']()}</p>
					{/if}
				{:else if view.section === 'slice' && activeDoc && activeSlice}
					<!-- the tool strip left of the canvas, the canvas fills the rest (UI-CONCEPTS E.4); the canvas is the group's strip –
					     every slice, the open one holds the selection (TEXT-ACROSS-SLICES.md §4) -->
					<div class="slice-room">
					<SliceToolbar {view} expressions={sliceHasExpressions(activeSlice)} />
					<div class="canvas-scroll" {@attach wheelZoom} data-testid="canvas-area">
						<StripCanvas
							bind:this={canvas}
							doc={activeDoc}
							{groupPath}
							active={view.sliceIndex}
							seams={view.seams}
							locale={displayLocale}
							grid={view.grid}
							tool={view.tool}
							layers={view.layers}
							selection={view.selection}
							live={liveRegion.is(SLICE_REGION)}
							zoom={view.zoom}
							pieceName={(p) => pieceLabel(p, choices?.registry, choices?.moods)}
							presets={collagePresets}
							onshown={(f) => (view.shownZoom = f)}
							onselect={(i, next) => {
								view.focusSlice(i);
								view.select(next);
							}}
							oncreate={(i, t, rect) => {
								view.focusSlice(i);
								CREATE[t](rect);
							}}
							oncentre={(i) => !view.selection && view.focusSlice(i)}
							onlongpress={(_i, _item, box) => (longMenu = box)}
						/>
					</div>
					</div>
				{:else}
					<p class="sc-empty pad">{m['creator.select_slice']()}</p>
				{/if}
			</main>

			<!-- Inspector -->
			{#if view.section === 'slice' && activeDoc && activeSlice}
				<Divider size={inspectorSize} edge="left" at="var(--inspector-width, 22rem)" label={m['creator.resize_panel']()} controls="slice-inspector" class="inspector-resize" />
				<aside class="inspector-region" id="slice-inspector" aria-label={m['creator.inspector']()} data-testid="inspector-region" data-sc-maximize={INSPECTOR}>
				<!-- phones: the sheet's grab handle (BACKLOG 10.50 e) – a separator like the Divider -->
				<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
				<div
					class="sheet-handle"
					role="separator"
					aria-orientation="horizontal"
					aria-label={m['creator.resize_panel']()}
					title={m['creator.resize_panel']()}
					tabindex="0"
					data-focus-own
					data-testid="sheet-handle"
					onpointerdown={sheetDown}
					onpointermove={sheetMove}
					onpointerup={sheetUp}
					onpointercancel={() => (sheetDrag = null)}
					onkeydown={sheetKey}
				><span></span></div>
				<!-- phones (E.10, variant C): the sheet's head = its tabs; closed, only this row shows -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="sheet-tabs" onpointerdown={tabsDown}>
					<BarTabs
						label={m['creator.slice_tab']()}
						value={sheetOpen ? view.inspectorTab : ''}
						onchange={(id) => pickSheetTab(id as InspectorTab)}
						tabs={[
							{ id: 'layers', icon: 'layers', label: m['creator.layers'](), testid: 'sheet-tab-layers' },
							{ id: 'audio', icon: 'music', label: m['creator.audio'](), testid: 'sheet-tab-audio' },
							{ id: 'events', icon: 'zap', label: m['creator.events'](), count: activeSlice.events?.length ?? 0, testid: 'sheet-tab-events' },
							{ id: 'slice', icon: 'image', label: m['creator.slice_tab'](), testid: 'sheet-tab-slice' }
						]}
					/>
				</div>
				<SliceInspector
					{view}
					groupTab={groupBody}
					{groupPath}
					boxOf={(i) => canvas?.drawnBox(i) ?? null}
					onpick={(i, next) => {
						view.focusSlice(i);
						view.select(next);
					}}
					onlongpress={(i, next, box) => {
						view.focusSlice(i);
						view.select(next);
						longMenu = box;
					}}
					doc={activeDoc}
					slice={activeSlice}
					{slicePath}
					group={activeGroup}
					{toon}
					{chapterNumber}
					seasonNumber={Number(season.get(season.primary, ['number'])) || 1}
					locale={displayLocale}
					{characters}
					{choices}
					presets={colorations}
					{groupLabels}
					chapterAudio={chapterData?.audio}
					{librarySample}
					{heldMusic}
					onaddtext={(preset) => addDialog(undefined, preset)}
					oncreatebox={createBoxForEvent}
					onremove={removeSelected}
					eventMark={(i) => markAt('groups', view.groupIndex, 'slices', view.sliceIndex, 'events', i)}
				>
					{#snippet after()}
						<MaximizeButton target={INSPECTOR} controls="slice-inspector" label={m['creator.maximize_panel']()} restoreLabel={m['creator.restore_panel']()} testid="inspector-maximize" />
					{/snippet}
				</SliceInspector>
				</aside>
			{/if}
		</div>
		{#if view.section === 'slice' && activeSlice}
			<!-- the item and piece pickers live outside the inspector: on a phone its sheet is display:none while closed, and a
			     modal <dialog> opened in there is an invisible 0×0 modal that makes the page inert (the item / piece tool's tap) -->
			<ItemPickerSheet bind:open={view.pickingItem} title={m['creator.add_slice_item']()} onpick={(item) => addSliceItem(item.id)} />
			<CollagePickerSheet bind:open={view.pickingPiece} {toon} registry={choices?.registry ?? null} moods={choices?.moods ?? defaultMoodVocabulary()} cast={choices?.cast} toons={choices?.toons} onpick={addPiece} />
		{/if}
	{/if}

</div>


{#snippet deskSliceNav()}
	{#if view.section === 'slice' && activeGroup?.slices?.length}
		<span class="slice-nav" role="group" aria-label={m['creator.slice_navigation']()}>
			<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex - 1)} disabled={view.sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid="prev-slice"><Icon name="chevron-left" /></button>
			<span class="slice-long">{m['creator.slice_of']({ index: view.sliceIndex + 1, count: activeGroup.slices.length })}</span><span class="slice-short">{view.sliceIndex + 1} / {activeGroup.slices.length}</span>
			<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex + 1)} disabled={view.sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid="next-slice"><Icon name="chevron-right" /></button>
		</span>
	{/if}
{/snippet}

<!-- the History panel, shown in the shell's right dock (useDock) – the Flow and Problems are FlowDock / ProblemsDock -->
{#snippet sidePanel()}
	<div class="side-panel">
		{#if open.history && activeDoc}
			<HistoryPanel doc={view.section === 'group' || view.section === 'slice' ? activeDoc : chapter} onclose={() => (open.history = false)} />
		{/if}
	</div>
{/snippet}

<style>
	/* DESIGN.md "Brutalist tools": the reader's type sizes, 2px ink frames, selected = inverted, ≥ 44px controls.
	   The AppShell runs in "fill" mode here: the editor is exactly the content area and scrolls its own columns. */
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}
	/* the bar's slice stepper (desktop): the count without its word below 1280 */
	.slice-short {
		display: none;
	}
	@media (max-width: 1279px) {
		.slice-long {
			display: none;
		}
		.slice-short {
			display: inline;
		}
	}
	.slice-nav {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		white-space: nowrap;
	}
	/* phones (BACKLOG 10.50 h): the content first – the chapter's groups, a group's slices */
	.phone-only {
		display: none;
	}
	@media (max-width: 767px) {
		.phone-only {
			display: block;
		}
	}
	.phone-groups {
		margin: calc(-1 * var(--space-1)) calc(-1 * var(--space-1)) var(--space-15);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.phone-groups .sc-nav-item {
		justify-content: space-between;
	}
	.phone-name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.phone-hint {
		flex: none;
		color: var(--muted);
	}
	@media (max-width: 767px) {
		.phone-slices {
			display: grid;
			grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
			gap: var(--space-05);
			margin-bottom: var(--space-15);
		}
	}
	.phone-slice {
		position: relative;
		display: block;
		width: 100%;
		aspect-ratio: 3 / 4;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		overflow: hidden;
	}
	/* a long press picks (no system menu, no drag ghost); a picked one: a thick frame, its number inverted */
	.phone-slice {
		-webkit-touch-callout: none;
		user-select: none;
	}
	.phone-group {
		-webkit-touch-callout: none;
		user-select: none;
	}
	.phone-slice.sc-selected {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--border-width-regular);
	}
	.phone-slice.sc-selected span {
		background: var(--primary-cyan);
		color: var(--ink);
	}
	.phone-slice img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.phone-slice span {
		position: absolute;
		left: 0;
		top: 0;
		padding: 0 0.25rem;
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	@media (max-width: 767px) {
		/* the inspector sheet's tabs are in the bottom bar (UI-CONCEPTS E.10) */
		.columns :global(.inspector > .sc-tabs) {
			display: none;
		}
	}
	/* status bars: ink on paper with a thick state rule (no tinted fills); unsaved = inverted */
	.columns {
		position: relative;
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
	}
	.columns.has-inspector {
		grid-template-columns: minmax(0, 1fr) var(--inspector-width, 22rem);
	}
	/* the inspector's region: its own scrolling, the border its Divider sits on */
	.inspector-region {
		min-width: 0;
		min-height: 0;
		overflow: auto;
		overscroll-behavior: contain;
		border-left: var(--border-width-regular) solid var(--ink);
	}
	/* narrower: the inspector goes below the canvas (sidebar 12rem + inspector 22rem + a 390px slice need ≈ 1280px) */
	@media (max-width: 1279px) {
		.columns.has-inspector {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) minmax(0, 50%);
		}
		.inspector-region {
			border-left: 0;
			border-top: var(--border-width-regular) solid var(--ink);
		}
		.columns :global(.inspector-resize) {
			display: none;
		}
	}
	/* phones: the inspector is a sheet below the canvas with a grab handle (BACKLOG 10.50 e); its head = the tabs (E.10) */
	.sheet-handle,
	.sheet-tabs {
		display: none;
	}
	@media (max-width: 767px) {
		/* open: the canvas row is as tall as the slice (at most the rest), the sheet starts right under it and takes the
		   free space, at least 45 % (owner 2026-10-18: "a straight cut after the line") */
		.columns.has-inspector {
			grid-template-rows: fit-content(100%) minmax(45%, 1fr);
		}
		/* dragged (and while dragging): exactly the dragged height, so it follows the finger – the canvas area gets the rest */
		.columns.has-inspector.sheet-sized {
			grid-template-rows: minmax(0, 1fr) minmax(0, var(--sheet-height));
		}
		.columns .main.fit {
			padding-bottom: 0;
		}
		/* inline-size, not size: the room's height follows the canvas (a phone's fit is the width, no cqh needed) */
		.columns .main.fit .canvas-scroll {
			container-type: inline-size;
		}
		.sheet-handle {
			position: sticky;
			top: 0;
			z-index: 2;
			display: flex;
			align-items: center;
			justify-content: center;
			height: 1.5rem;
			background: var(--paper);
			cursor: ns-resize;
			touch-action: none;
		}
		.sheet-handle span {
			width: 3rem;
			height: 0.3125rem;
			border-radius: 0.25rem;
			background: var(--ink);
		}
		.sheet-handle:focus-visible {
			outline: var(--border-width-focus) solid var(--ink);
			outline-offset: calc(-1 * var(--border-width-focus));
		}
		.inspector-region :global(.inspector) {
			padding-top: 0;
		}
		.sheet-tabs {
			touch-action: none;
			position: sticky;
			top: 0;
			z-index: 2;
			display: flex;
			background: var(--paper);
			border-bottom: var(--border-width-regular) solid var(--ink);
		}
		/* variant C: no grab handle – the tabs open and close the sheet */
		.sheet-handle {
			display: none;
		}
		/* closed (variant C): the tab row only, on the bottom bar; the free space goes to the canvas area (owner
		   2026-10-18: a closed sheet must not look open and empty) */
		.columns.has-inspector.sheet-closed {
			grid-template-rows: minmax(0, 1fr) auto;
		}
		.sheet-closed .inspector-region {
			overflow: hidden;
		}
		/* closed, the tab row sits on the bottom bar: its rule is the bar's */
		.sheet-closed .sheet-tabs {
			border-bottom: 0;
		}
		/* :global – the inspector is SliceInspector's root, scoped CSS here would miss it (the sheet then grew with it) */
		.sheet-closed .inspector-region > :global(:not(.sheet-tabs)) {
			display: none;
		}
		/* the strip editor, E3: the tools are the bottom row (PhoneKinds) – no tool strip left of the canvas, no tab row on
		   the sheet; the sheet opens from the row, its grab handle drags it, closed it takes no room */
		.slice-room :global(.slice-tools) {
			display: none;
		}
		.sheet-tabs {
			display: none;
		}
		.columns.has-inspector:not(.sheet-closed) .sheet-handle {
			display: flex;
		}
		.columns.has-inspector.sheet-closed {
			grid-template-rows: minmax(0, 1fr) 0;
		}
		.sheet-closed .inspector-region {
			border-top: 0;
		}
	}
	/* the inspector maximized: it covers the columns (beside or below the canvas), the canvas stays laid out, hidden */
	.columns.inspector-max .main {
		grid-area: 1 / 1;
		visibility: hidden;
	}
	.columns.inspector-max .inspector-region {
		grid-area: 1 / 1 / -1 / -1;
		z-index: 16;
		border: 0;
		background: var(--paper);
	}
	.columns.inspector-max :global(.inspector-resize) {
		display: none;
	}
	.main {
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		min-width: 0;
	}
	/* the phone's top bar in the crumb row (E3): equal squares, borderless */
	.phone-top {
		display: flex;
		align-items: center;
	}
	.phone-top .sq,
	.phone-top :global(.trigger) {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		min-width: 0;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	.phone-top .sq[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.phone-top .badge,
	.phone-top .dot {
		position: absolute;
		top: var(--space-025);
		right: var(--space-025);
		font-size: var(--type-caption-size);
		line-height: 1;
	}
	.phone-top .badge {
		padding: 0 var(--space-025);
		background: var(--state-warning);
		color: var(--ink);
	}
	.canvas-scroll {
		overflow: auto;
		padding: var(--space-1) 0 var(--space-15);
	}
	/* the slice editor (UI-CONCEPTS E.4): the tool strip left, the slice fills the rest – fit = the column's height (at most
	   2x the slice), zoomed = the area scrolls (owner 2026-10-03 + 2026-10-07) */
	.main.fit {
		display: flex;
		flex-direction: column;
		padding-bottom: var(--space-1);
	}
	.slice-room {
		flex: 1;
		display: flex;
		gap: var(--space-05);
		min-height: 12rem;
	}
	.main.fit .canvas-scroll {
		flex: 1;
		min-width: 0;
		padding: 0;
		container-type: size;
	}
	.sub {
		margin-top: var(--space-15);
	}
	/* the open page panel (in the shell's right dock: it sizes and places it) */
	.side-panel {
		box-sizing: border-box;
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
	}
	.pad {
		padding: var(--space-1);
	}
	.error {
		color: var(--error);
	}
</style>
