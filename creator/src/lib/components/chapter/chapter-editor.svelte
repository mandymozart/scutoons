<script lang="ts">
	import type { VariantMedia } from '@scutoons/media';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { onMount, tick, untrack } from 'svelte';
	import { ActionBar, confirmDialog, Divider, Icon, MaximizeButton, panelWidth, shellDock, useDock } from '@scutoons/ui';
	import { liveRegion, region } from '@scutoons/ui';
	import { beforeNavigate, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { dockWidth, flowWidth, usePageNav } from '$lib/nav.svelte.ts';
	import { fileLabel, sceneLabel } from '$lib/labels.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import { editorKeys, isField } from '$lib/editor/editor-keys.ts';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { sceneDoc, toonDoc } from '$lib/editor/languages.ts';
	import { entities, getAt, deleteAt, Renderer, sceneNameOfRef, scriptLabels, type Path, type ToonLanguages } from '@scutoons/content';
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
	import SliceCanvas from '$lib/components/slice/slice-canvas.svelte';
	import { legacyTextCount, legacyPlan, applyLegacyPlan, type TextPreset } from '$lib/editor/text-presets.ts';
	import { itemCatalog } from '$lib/editor/catalog.svelte.ts';
	import { ChapterView, clampZoom, type Section } from '$lib/editor/chapter-state.svelte.ts';
	import * as edits from '$lib/editor/slice-edit.ts';
	import { clipboardEvents, useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { sliceClipHandler } from '$lib/editor/slice-clip.ts';
	import { setClipContext } from '$lib/editor/clip-context.ts';
	import { countPlacements } from '$lib/editor/clip.ts';
	import { itemName } from '$lib/editor/catalog.svelte.ts';
	import { api } from '$lib/api.ts';
	import type { ClipResult } from '$lib/editor/clipboard.svelte.ts';
	import ChapterTree from '$lib/components/chapter/chapter-tree.svelte';
	import SliceToolbar from '$lib/components/slice/slice-toolbar.svelte';
	import SliceInspector from '$lib/components/slice/slice-inspector.svelte';
	import { mapDoc as worldMapDoc } from '$lib/editor/map.ts';
	import { chapterSlices } from '@scutoons/content';
	import MetaPanel from '$lib/components/chapter/meta-panel.svelte';
	import FeaturingField from '$lib/components/fields/featuring-field.svelte';
	import RelatedItemsField from '$lib/components/fields/related-items-field.svelte';
	import GroupPanel from '$lib/components/chapter/group-panel.svelte';
	import StripPreview from '$lib/components/chapter/strip-preview.svelte';
	import AudioPanel from '$lib/components/chapter/audio-panel.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsDock from '$lib/components/page/problems-dock.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import FlowDock from '$lib/components/flow/flow-dock.svelte';
	import ReaderDock from '$lib/components/reader/reader-dock.svelte';
	import LiveChip from '$lib/components/status/live-chip.svelte';
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
	let refs = $state<Record<string, ContentDoc>>({});
	// the season's cast + the character registry: the VN character fields pick from every character (the season's cast
	// first); picking someone who isn't cast adds them to the season's cast (saved / published with the chapter)
	/** the chapter's season (the editor edits season 1) */
	const SEASON = 1;
	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	const moods = moodsDoc();
	// the world map: a slice unlocks places (agents/MAP.md, edited here and in the map builder – one storage, the place's
	// `unlock`); saved / published with the chapter once edited here
	const worldMap = worldMapDoc();
	let mapTouched = $state(false);
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
	/** the chapter's groups with each scene document (source language) in place of its ref – what the VN plays */
	const sceneGroups = () => groups.map((g) => (g?.ref?.src ? (refs[g.ref.src]?.data?.[refs[g.ref.src].primary] ?? {}) : g));
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
		const all = sceneGroups();
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
		...Object.values(refs),
		...(castTouched || cast.dirty ? [cast] : []),
		...(mapTouched || worldMap.dirty ? [worldMap] : [])
	]);
	const anyDirty = $derived(allDocs.some((d) => d.dirty));
	/** the chapter's documents with slices (the chapter + its separate scenes) */
	const sliceDocs = $derived<ContentDoc[]>([chapter, ...Object.values(refs)]);
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

	async function ensureRef(src: string) {
		if (refs[src]) return refs[src];
		const doc = sceneDoc(toon, src, languages);
		refs[src] = doc;
		await doc.load();
		return doc;
	}

	onMount(async () => {
		await Promise.all([chapter.load(), season.load(), cast.load(), registry.load(), moods.load(), worldMap.load()]);
		loadToons(getLocale()).then((list) => (toonInfos = list));
		await Promise.all(groups.filter((g) => g?.ref?.src).map((g) => ensureRef(g.ref.src)));
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
		const snapshot = $state.snapshot(sceneGroups());
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
	// and the keyboard write it; opening a group loads its separate scene file
	const view = new ChapterView((index) => {
		const src = groups[index]?.ref?.src;
		if (src) ensureRef(src);
	});
	// the canvas, the strip preview and the inspector show the top bar's TEXT language (default the source; owner
	// 2026-10-06: one language everywhere, the header's switch the only one)
	const displayLocale = $derived(textLanguage.current(languages.languages, languages.source));
	// snap, grid size and zoom are remembered on this device
	$effect(() => {
		void view.snap;
		void view.gridSize;
		void view.zoom;
		view.savePrefs();
	});
	// the Flow / Problems the last page docked stay open here (taken over, not remounted – UI-CONCEPTS E.3 b); `?flow=1`
	// (a deep link) opens the Flow
	let panel = $state<'problems' | 'history' | 'flow' | 'reader' | null>(
		page.url.searchParams.get('flow') === '1' || shellDock.has('flow')
			? 'flow'
			: shellDock.has('problems')
				? 'problems'
				: shellDock.has('reader')
					? 'reader'
					: null
	);
	// docked in the shell's right dock (agents/DOCKING.md): the page makes room; with Ask open too the sidebar folds. The
	// Flow and Problems are mounted by the shell (`kind` + `content`): the next page of the same kind takes them over
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
		onclose: () => (panel = null)
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
		onclose: () => (panel = null)
	});
	const problemsProps = () => ({ problems, docs: allDocs, home: chapter.entity, ongoto: gotoProblem, onclose: () => (panel = null) });
	useDock(() => {
		if (!panel) return null;
		const docked = { size: panel === 'flow' ? flowSize : panel === 'reader' ? readerSize : panelSize, title: PANELS[panel].title(), icon: PANELS[panel].icon, close: () => (panel = null) };
		if (panel === 'flow') return { ...docked, kind: 'flow', content: { component: FlowDock, props: flowProps } };
		if (panel === 'problems') return { ...docked, kind: 'problems', content: { component: ProblemsDock, props: problemsProps } };
		if (panel === 'reader') return { ...docked, kind: 'reader', content: { component: ReaderDock, props: readerProps } };
		return { ...docked, panel: sidePanel };
	});
	function togglePanel(next: 'problems' | 'history' | 'flow' | 'reader') {
		panel = panel === next ? null : next;
	}
	let canvas = $state<SliceCanvas | null>(null);
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
		beforeSave: () => {
			// the chapter file's (deprecated) status mirrors the season, its old `published` flag goes
			syncChapter(chapter);
			// a cast saved from here keeps the toon's star in every season
			if (castTouched || cast.dirty) castWithStar(cast, registry, toon, [SEASON]);
		},
		afterSave: () => refreshImages(),
		errorCount: () => errorCount
	});

	const currentGroup = $derived(groups[view.groupIndex]);
	const refSrc = $derived<string | null>(currentGroup?.ref?.src ?? null);
	const activeDoc = $derived<ContentDoc | null>(refSrc ? (refs[refSrc] ?? null) : chapter);
	/**
	 * undo / redo act on the file edited last (owner 2026-10-03: moving groups edits the chapter while a scene file is
	 * open) – the open group's file or the chapter; redo on the file that was undone last
	 */
	const undoDoc = $derived<ContentDoc | null>(
		[activeDoc, chapter].filter((d): d is ContentDoc => !!d?.canUndo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0] ?? null
	);
	let undoneDoc = $state<ContentDoc | null>(null);
	const redoDoc = $derived<ContentDoc | null>(undoneDoc?.canRedo ? undoneDoc : activeDoc?.canRedo ? activeDoc : chapter.canRedo ? chapter : null);
	function undoLast() {
		const d = undoDoc;
		if (!d) return;
		d.undo();
		undoneDoc = d;
	}
	const redoLast = () => redoDoc?.redo();
	const groupPath = $derived<Path>(refSrc ? [] : ['groups', view.groupIndex]);
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
	const groupKey = (g: any, i: number) => g?.label ?? (g?.ref?.src ? sceneNameOfRef(g.ref.src) : null) ?? String(i + 1);
	/** a separate scene's own label (stored in the scene; owner 2026-10-03: renamed scene-13 still showed "Scene 14") */
	const refLabel = (src: string): string | undefined => {
		const doc = refs[src];
		const label = doc && !doc.loading ? doc.get(doc.primary, ['label']) : undefined;
		return typeof label === 'string' && label.trim() ? label : undefined;
	};
	const groupName = (g: any, i: number) =>
		g?.ref?.src ? (refLabel(g.ref.src) ?? sceneLabel(toon, g.ref.src)) : (g?.label ?? `${m['creator.groups']()} ${i + 1}`);
	/** the ActionBar's not-live chip (E.1): the chapter, the scene while one of its parts is open (owner 2026-10-03:
	 * "scene-14" next to Meta confused), the season while it has something unpublished */
	const liveFiles = $derived([
		{ doc: chapter, label: fileLabel(chapter.entity) },
		...(refSrc && refs[refSrc] && (view.section === 'group' || view.section === 'slice') ? [{ doc: refs[refSrc], label: refLabel(refSrc) ?? sceneLabel(toon, refSrc) }] : []),
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
				// the chapter's visibility: its entry in the season (null = none)
				seasonStatus: doc === chapter ? seasonStatusOf(season, languages.source, chapterNumber) : undefined,
				// linked groups: a ref group's `continues` lives in its scene
				resolveGroup: (g: any) => (g?.ref?.src ? refs[g.ref.src]?.data?.[refs[g.ref.src].primary] : g),
				// reachability: what the rest of the toon jumps to / sets
				facts,
				// jump targets: the toon / chapter / label they name exist (as far as loaded)
				targets: jumpPlaces.targets(),
				// placed items (and related items) are checked against the catalog once it is loaded
				items: catalogItems,
				itemRespawn
			};
			if (data == null) {
				if (doc.source[doc.primary] === 'missing') out.push({ level: 'error', file: doc.entity, locale: null, path: [], message: m['creator.missing']() });
				continue;
			}
			// chapter, scenes, season: one language-neutral document, checked once + its texts per language
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
		if (!chapter.loading && hasCast) out.push(...featuringSuggestions(chapterData?.featuring, chapterSceneCharacters(sceneGroups()), castIndex(castOf(cast), SEASON), chapter.entity));
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
		if (p.file !== chapter.entity) {
			const gi = groups.findIndex((g) => g?.ref?.src === p.file);
			if (gi < 0) return;
			view.selectGroup(gi);
		} else if (path[0] === 'groups' && typeof path[1] === 'number') {
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
	const jumpPlaces = setJumpPlaces(new JumpPlaces(toon, chapterNumber, () => sceneGroups().map((g) => g?.label).filter((l): l is string => typeof l === 'string' && !!l)));
	$effect(() => {
		jumpPlaces.toons = toonInfos.length ? toonInfos.map((t) => ({ id: t.id, name: t.name })) : null;
	});
	// load what the chapter's targets name (other chapters' / toons' labels) – once per target, debounced while editing
	$effect(() => {
		const docs = sliceDocs.map((d) => $state.snapshot(d.data[d.primary]));
		const timer = setTimeout(() => jumpPlaces.prefetch(docs), 500);
		return () => clearTimeout(timer);
	});
	/** every text box id of the chapter and its scenes: a new box gets a fresh one (owner 2026-10-06: unique in the chapter) */
	const chapterDialogIds = () =>
		new Set<string>(sceneGroups().flatMap((g: any) => (g?.slices ?? []).flatMap((s: any) => (s?.dialog ?? []).map((d: any) => d?.id).filter((id: unknown) => typeof id === 'string'))));
	const sliceTarget = (): edits.SliceTarget | null =>
		activeDoc && view.sliceIndex >= 0 ? { doc: activeDoc, slicePath, chapterNumber, sliceIndex: view.sliceIndex, slice: activeSlice, taken: chapterDialogIds } : null;

	// linked groups (owner 2026-10-06): a group whose script can jump away and that continues – the next group needs a
	// label, else nothing leads back to it (a reminder with one click, next to the script)
	const nextGroup = $derived(sceneGroups()[view.groupIndex + 1]);
	const needsNextLabel = $derived(!!activeGroup?.continues && scriptJumps(activeGroup) && !!nextGroup && !nextGroup.label);
	function labelNextGroup() {
		const index = view.groupIndex + 1;
		const raw = groups[index];
		const taken = new Set(sceneGroups().map((g: any) => g?.label).filter(Boolean));
		let label = raw?.ref?.src ? sceneNameOfRef(raw.ref.src) : `scene-${index + 1}`;
		for (let n = 2; !label || taken.has(label); n++) label = `scene-${index + 1}-${n}`;
		if (raw?.ref?.src) refs[raw.ref.src]?.edit((file) => void (file.label = label));
		else chapter.edit((file) => void (file.groups[index].label = label));
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
	function removeSelected() {
		const t = sliceTarget();
		if (!t || !view.selection) return;
		edits.removeSelected(t, view.selection);
		view.selection = null;
	}

	const previewLabel = $derived(view.section === 'group' || view.section === 'slice' ? (activeGroup?.label ?? '') : '');
	/**
	 * The chapter in the live reader with its draft preview on (`?preview=draft`, viewer lib/draft-preview.svelte.ts):
	 * the saved drafts, at the open group. The locale prefix picks the text language; the reader has de and en only.
	 */
	const readerLocales = $derived(chapter.textLanguages.filter((l) => l === 'de' || l === 'en'));
	const previewHref = (locale: string) =>
		`${VIEWER_URL}/${locale}/${toon}/chapter/${chapterNumber}?preview=draft${previewLabel ? `&label=${encodeURIComponent(previewLabel)}` : ''}`;
	/** unsaved changes would be missing in the reader: save the draft first (with the note dialog), then open it */
	async function openPreview(event: MouseEvent, locale: string) {
		if (!anyDirty) return;
		event.preventDefault();
		const save = await confirmDialog({
			title: m['creator.preview_unsaved_title'](),
			message: m['creator.preview_unsaved_message'](),
			confirmLabel: m['creator.preview_unsaved_save']()
		});
		if (!save) return;
		await session.saveDraft();
		if (anyDirty) return;
		// a popup blocker may refuse the window after the dialogs: then a hint (not 'noopener': that always returns null)
		const tab = window.open(previewHref(locale), '_blank');
		if (tab) tab.opener = null;
		else session.notify(m['creator.preview_saved_open']());
	}

	// ── Guards, autosave, keyboard ──────────────────────────────────────────────────────────────────────
	// unsaved changes: leaving asks in-page (preview links open in a new tab); lib/editor/ask.ts
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
	// this chapter's documents live, every other chapter / scene from the API (GET /api/catalog/items/<id>/placements,
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
	/** per limited item: its limit, the placements in this chapter's documents and elsewhere (other chapters / scenes) */
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
			groupLabels: () => sceneGroups().map((g: any) => g?.label).filter((l: unknown): l is string => typeof l === 'string' && !!l),
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

<svelte:window onkeydown={onKeydown} />
<!-- right-click / long-press in a copy region: Copy · Cut · Paste · Paste settings (BACKLOG 10.39 step 8) -->
<ClipMenu />

<!-- the Flow panel opens where the Ask panel does: a round button in the top bar (owner 2026-10-06); the Reader next to it
     (BACKLOG 10.49) -->
{#snippet flowToggle()}
	<button type="button" class="sc-round" aria-pressed={panel === 'reader'} title={m['creator.reader_open']()} aria-label={m['creator.reader_open']()} onclick={() => togglePanel('reader')} data-testid="reader-toggle"><Icon name="smartphone" /></button>
	<button type="button" class="sc-round" aria-pressed={panel === 'flow'} title={m['creator.flow_open']()} aria-label={m['creator.flow_open']()} onclick={() => togglePanel('flow')} data-testid="flow-toggle"><Icon name="git-merge" /></button>
{/snippet}

{#snippet tree()}
	<ChapterTree {view} {chapter} {groups} {groupName} {activeDoc} {groupPath} {chapterNumber} {markAt} resolveGroup={(g) => (g?.ref?.src ? refs[g.ref.src]?.data?.[refs[g.ref.src].primary] : g)} />
{/snippet}

<div class="editor">

	<!-- Bottom action bar (PLATFORM.md "Bottom action bar"): navigation + validation summary · tools · save / publish -->
	<!-- prev / next slice: in the bar's end from 768px (always at hand), in "More" on phones (the end holds only save) -->
	{#snippet sliceNav(where: 'start' | 'end')}
		{#if view.section === 'slice' && activeGroup?.slices?.length}
			<span class="slice-nav in-{where}" role="group" aria-label={m['creator.slice_navigation']()}>
				<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex - 1)} disabled={view.sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid={where === 'end' ? 'prev-slice' : undefined}><Icon name="chevron-left" /></button>
				<span class="slice-long">{m['creator.slice_of']({ index: view.sliceIndex + 1, count: activeGroup.slices.length })}</span><span class="slice-short">{view.sliceIndex + 1} / {activeGroup.slices.length}</span>
				<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex + 1)} disabled={view.sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid={where === 'end' ? 'next-slice' : undefined}><Icon name="chevron-right" /></button>
			</span>
		{/if}
	{/snippet}
	<!-- the reader's type size needs the room: below 2048px start + secondary sit in "More" (DESIGN.md "Brutalist tools") -->
	<ActionBar
		label={m['creator.page_actions']()}
		moreLabel={m['creator.more']()}
		breakpoint={2048}
	>
		{#snippet start()}
			<!-- phones have no Cmd keys: copy / cut / paste on the live region from here (BACKLOG 10.39) -->
			<span class="clip-phone"><ClipMenu bar /></span>
			<!-- phones: slice nav, problems, undo / redo are in the slice bar above (BACKLOG 10.50 c) -->
			<button class="ghost not-phone" class:active={panel === 'problems'} onclick={() => togglePanel('problems')} data-testid="problems-toggle">
				{m['creator.problems']()} <span class:bad={errorCount > 0}>{errorCount}</span>/<span>{warningCount}</span>
			</button>
		{/snippet}
		{#snippet secondary()}
			<button class="ghost not-phone" title="{m['creator.undo']()} (Ctrl+Z)" aria-label={m['creator.undo']()} disabled={!undoDoc} onclick={undoLast} data-testid="undo"><Icon name="rotate-ccw" /></button>
			<button class="ghost not-phone" title="{m['creator.redo']()} (Ctrl+Shift+Z)" aria-label={m['creator.redo']()} disabled={!redoDoc} onclick={redoLast} data-testid="redo"><Icon name="rotate-cw" /></button>
			<span class="preview">
				{m['creator.preview']()}:
				{#each readerLocales as l}
					<a href={previewHref(l)} target="_blank" rel="noopener" onclick={(e) => openPreview(e, l)} title={m['creator.preview_in_reader']({ locale: l })} data-testid="preview-{l}">{l}</a>
				{/each}
			</span>
			<button class="ghost" class:active={panel === 'history'} onclick={() => togglePanel('history')}>{m['creator.history']()}</button>
			<input class="message in-start" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
		{/snippet}
		{#snippet end()}
			{@render sliceNav('end')}
			<input class="message in-end" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
			<button onclick={() => session.saveDraft()} disabled={!anyDirty || anySaving} data-testid="save-draft">
				{#if anySaving}{m['creator.saving']()}{:else}<span class="save-long">{m['creator.save_draft']()}</span><span class="save-short">{m['creator.save_short']()}</span>{/if}{anyDirty ? ' •' : ''}
			</button>
			<LiveChip files={liveFiles} />
			<button class="secondary" onclick={() => session.publishAll()} disabled={anySaving} data-testid="publish">{publishLabel(toon)}</button>
		{/snippet}
	</ActionBar>

	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		<DocNotices {session} docs={allDocs} />

		<!-- the slice canvas + its inspector = ONE selection region (live on load; the keys act on it) -->
		<div class="columns" use:region={view.section === 'slice' ? { name: SLICE_REGION, initial: true } : null} class:has-inspector={view.section === 'slice'} class:inspector-max={inspectorMax && view.section === 'slice'} bind:this={columnsEl} style:--inspector-width={inspectorSize.css} style:--sheet-height={sheetRem === null ? null : `${sheetRem}rem`}>
			<!-- Main -->
			<main class="main" class:fit={view.section === 'slice' && !!activeSlice}>
				{#if view.section === 'meta'}
					<!-- phones (BACKLOG 10.50 h): the chapter leads with its groups – the tree is in the drawer there -->
					<nav class="phone-only phone-groups" aria-label={m['creator.groups']()} data-testid="phone-groups">
						<p class="sc-nav-head">{m['creator.groups']()}</p>
						{#each sceneGroups() as g, gi (gi)}
							<button type="button" class="sc-nav-item" onclick={() => view.selectGroup(gi)} data-testid="phone-group-{gi}">
								<span class="phone-name">{groupName(groups[gi], gi)}</span>
								<span class="phone-hint">{g?.slices?.length ?? 0}{g?.events ? ' · VN' : ''}</span>
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
									{sceneGroups}
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
									<button type="button" class="phone-slice" onclick={() => view.selectSlice(si)} aria-label={m['creator.slice_n']({ number: si + 1 })} title={m['creator.slice_n']({ number: si + 1 })} data-control-exempt="slice thumbnail" data-testid="phone-slice-{si}">
										<img src={toonUrl(activeDoc.toon, s?.image)} alt="" loading="lazy" />
										<span>{si + 1}</span>
									</button>
								{/each}
							</nav>
						{/if}
						<GroupPanel doc={activeDoc} {groupPath} {refSrc} {toon} {chapterNumber} {groupLabels} last={view.groupIndex === groups.length - 1} />
						<!-- the reader's screen: a full-width horizontal group sideways, a vertical group stacked (UI-CONCEPTS E.6) -->
						{#if activeGroup?.renderer?.type === Renderer.HORIZONTAL && activeGroup.renderer.params?.fullWidth}
							<StripPreview doc={activeDoc} {groupPath} locale={displayLocale} onopen={(i) => view.selectSlice(i)} />
						{:else if activeGroup?.renderer?.type === Renderer.VERTICAL}
							<StripPreview doc={activeDoc} {groupPath} locale={displayLocale} vertical onopen={(i) => view.selectSlice(i)} />
						{/if}
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
					{:else if activeDoc?.error}
						<p class="error">{activeDoc.error}</p>
					{:else}
						<p>{m['creator.loading']()}</p>
					{/if}
				{:else if view.section === 'slice' && activeDoc && activeSlice}
					<!-- the tool strip left of the canvas, the canvas fills the rest (UI-CONCEPTS E.4) -->
					<div class="slice-room">
					<SliceToolbar {view} expressions={sliceHasExpressions(activeSlice)} />
					<div class="canvas-scroll" {@attach wheelZoom} data-testid="canvas-area">
						<SliceCanvas
							bind:this={canvas}
							doc={activeDoc}
							{slicePath}
							locale={displayLocale}
							grid={view.grid}
							tool={view.tool}
							layers={view.layers}
							selection={view.selection}
							live={liveRegion.is(SLICE_REGION)}
							zoom={view.zoom}
							onshown={(f) => (view.shownZoom = f)}
							onselect={(next) => view.select(next)}
							oncreate={(t, rect) => (t === 'text' ? addDialog(rect) : t === 'item' ? view.pickItem({ x: rect.x, y: rect.y }) : t === 'event' ? addSliceEvent(rect) : addSampleArea(rect))}
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
				<SliceInspector
					{view}
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
					{groupLabels}
					chapterAudio={chapterData?.audio}
					{librarySample}
					{heldMusic}
					{worldMap}
					chapterSlices={chapterSlices(sceneGroups())}
					onmaptouch={() => (mapTouched = true)}
					onaddtext={(preset) => addDialog(undefined, preset)}
					onadditem={addSliceItem}
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
		<!-- phones (BACKLOG 10.50 c, the approved draft): the slice bar above the ActionBar – ‹ Slice n/m ›, undo, redo, the
		     problems (opens Problems); round icon buttons, More keeps the rest -->
		<div class="slice-bar" role="toolbar" aria-label={m['creator.slice_navigation']()} data-testid="slice-bar">
			{#if view.section === 'slice' && activeGroup?.slices?.length}
				<button type="button" class="sc-round" onclick={() => view.selectSlice(view.sliceIndex - 1)} disabled={view.sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid="bar-prev-slice"><Icon name="chevron-left" /></button>
				<span class="bar-count" data-testid="bar-slice-count">{m['creator.slice_of']({ index: view.sliceIndex + 1, count: activeGroup.slices.length })}</span>
				<button type="button" class="sc-round" onclick={() => view.selectSlice(view.sliceIndex + 1)} disabled={view.sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid="bar-next-slice"><Icon name="chevron-right" /></button>
			{/if}
			<span class="bar-spacer"></span>
			<button type="button" class="sc-round" title={m['creator.undo']()} aria-label={m['creator.undo']()} disabled={!undoDoc} onclick={undoLast} data-testid="bar-undo"><Icon name="rotate-ccw" /></button>
			<button type="button" class="sc-round" title={m['creator.redo']()} aria-label={m['creator.redo']()} disabled={!redoDoc} onclick={redoLast} data-testid="bar-redo"><Icon name="rotate-cw" /></button>
			<button
				type="button"
				class="sc-round bar-problems"
				class:is-bad={errorCount > 0}
				aria-pressed={panel === 'problems'}
				title="{m['creator.problems']()} {errorCount}/{warningCount}"
				aria-label="{m['creator.problems']()} {errorCount}/{warningCount}"
				onclick={() => togglePanel('problems')}
				data-testid="bar-problems"
				><Icon name="alert-triangle" />{#if errorCount + warningCount}<span class="corner">{errorCount || warningCount}</span>{/if}</button
			>
		</div>
	{/if}

</div>


<!-- the History panel, shown in the shell's right dock (useDock) – the Flow and Problems are FlowDock / ProblemsDock -->
{#snippet sidePanel()}
	<div class="side-panel">
		{#if panel === 'history' && activeDoc}
			<HistoryPanel doc={view.section === 'group' || view.section === 'slice' ? activeDoc : chapter} onclose={() => (panel = null)} />
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
	/* the bar fits at every width ≥ 375 (owner screenshot 2026-10-07: the live chip pushed Publish out at 1440 – the
	   reader's type size): below 1280 "Save" + the slice count without its word + the note in "More"; below 768 the
	   bar holds only save / publish (slice nav and note in "More"). The chip drops its state word below 1600 (live-chip). */
	.save-short,
	.slice-short {
		display: none;
	}
	@media (max-width: 1279px) {
		.save-long,
		.slice-long {
			display: none;
		}
		.save-short,
		.slice-short {
			display: inline;
		}
	}
	/* the clipboard buttons only where there are no keys: phones (in "More") */
	.clip-phone {
		display: none;
	}
	@media (max-width: 767px) {
		.clip-phone {
			display: contents;
		}
	}
	.slice-nav {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		white-space: nowrap;
	}
	/* the note gives way first: it shrinks (ActionBar's end may shrink) before anything is pushed out */
	.message {
		flex: 0 1 10rem;
		width: 10rem !important;
		min-width: 4rem !important;
	}
	/* below 1024 the slice nav keeps its arrows (the count is in the crumbs) */
	@media (max-width: 1023px) {
		.slice-nav.in-end .slice-short {
			display: none;
		}
	}
	@media (max-width: 1279px) {
		.message.in-end {
			display: none !important;
		}
		.message.in-start {
			width: 100% !important;
		}
	}
	@media (min-width: 1280px) {
		.message.in-start {
			display: none !important;
		}
	}
	@media (max-width: 767px) {
		.in-end {
			display: none !important;
		}
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
	/* phones: the slice bar above the ActionBar (BACKLOG 10.50 c) – its controls leave "More" */
	.slice-bar {
		display: none;
	}
	@media (max-width: 767px) {
		.not-phone {
			display: none !important;
		}
		.slice-bar {
			flex: none;
			display: flex;
			align-items: center;
			gap: var(--space-05);
			padding: var(--space-025) var(--space-1);
			border-top: var(--border-width-regular) solid var(--ink);
			background: var(--paper);
		}
		/* the toasts sit above the slice bar too */
		:global(body:has(.slice-bar)) {
			--sc-above-action-bar: calc(var(--control-height) + 2 * var(--space-025) + var(--border-width-regular));
		}
	}
	.bar-count {
		white-space: nowrap;
	}
	.bar-spacer {
		flex: 1;
	}
	.bar-problems {
		position: relative;
	}
	.bar-problems .corner {
		position: absolute;
		right: -0.25rem;
		bottom: -0.25rem;
		min-width: 1.25rem;
		padding: 0 0.25rem;
		box-sizing: border-box;
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.bar-problems.is-bad .corner {
		background: var(--error);
		border-color: var(--error);
		color: var(--paper);
	}
	.preview {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	.preview a {
		color: var(--ink);
	}
	.bad {
		color: var(--error);
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
	/* phones: the inspector is a sheet below the canvas with a grab handle (BACKLOG 10.50 e) */
	.sheet-handle {
		display: none;
	}
	@media (max-width: 767px) {
		.columns.has-inspector {
			grid-template-rows: minmax(0, 1fr) minmax(0, var(--sheet-height, 45%));
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
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
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
